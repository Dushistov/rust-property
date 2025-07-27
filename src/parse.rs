// Copyright (C) 2019-2021 Boyu Yang
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use quote::{quote, ToTokens};
use syn::{
    meta::ParseNestedMeta, parse::Result as ParseResult, punctuated::Punctuated, spanned::Spanned,
    Error as SynError, LitStr, Meta, Token,
};

const ATTR_NAME: &str = "property";
const SKIP: &str = "skip";

pub(crate) struct ContainerDef {
    pub name: syn::Ident,
    pub generics: syn::Generics,
    pub fields: Vec<FieldDef>,
}

pub(crate) struct FieldDef {
    pub ident: syn::Ident,
    pub ty: syn::Type,
    pub conf: FieldConf,
}

#[derive(Clone, Copy)]
pub(crate) enum GetTypeConf {
    Auto,
    Ref,
    Copy,
    Clone,
}

#[derive(Clone, Copy)]
pub(crate) enum SetTypeConf {
    Ref,
    Own,
    None,
    Replace,
}

#[derive(Clone, Copy)]
pub(crate) enum ClrScopeConf {
    Auto,
    Option,
    All,
}

#[derive(Clone, Copy)]
pub(crate) enum VisibilityConf {
    Disable,
    Public,
    Crate,
    Private,
}

#[derive(Clone)]
pub(crate) enum MethodNameConf {
    Name(String),
    Format { prefix: String, suffix: String },
}

#[derive(Clone)]
pub(crate) struct GetFieldConf {
    pub vis: VisibilityConf,
    pub name: MethodNameConf,
    pub typ: GetTypeConf,
}

#[derive(Clone)]
pub(crate) struct SetFieldConf {
    pub vis: VisibilityConf,
    pub name: MethodNameConf,
    pub typ: SetTypeConf,
    pub full_option: bool,
}

#[derive(Clone)]
pub(crate) struct MutFieldConf {
    pub vis: VisibilityConf,
    pub name: MethodNameConf,
}

#[derive(Clone)]
pub(crate) struct ClrFieldConf {
    pub vis: VisibilityConf,
    pub name: MethodNameConf,
    pub scope: ClrScopeConf,
}

trait ParseFieldConf {
    fn set_option(&mut self, nested_meta: ParseNestedMeta) -> ParseResult<()>;
}

impl ParseFieldConf for GetFieldConf {
    fn set_option(&mut self, nested_meta: ParseNestedMeta) -> ParseResult<()> {
        if let Some(vis) = VisibilityConf::from_meta(&nested_meta) {
            self.vis = vis;
        } else if let Some(name) = MethodNameConf::from_meta(&nested_meta)? {
            self.name = name;
        } else if nested_meta.path.is_ident("type") {
            let value = expected_value(&nested_meta)?;
            self.typ = GetTypeConf::from_value(&value)?;
        } else {
            return Err(SynError::new(
                nested_meta.path.span(),
                "this attribute was unknown",
            ));
        }

        Ok(())
    }
}

impl ParseFieldConf for SetFieldConf {
    fn set_option(&mut self, nested_meta: ParseNestedMeta) -> ParseResult<()> {
        if let Some(vis) = VisibilityConf::from_meta(&nested_meta) {
            self.vis = vis;
        } else if let Some(name) = MethodNameConf::from_meta(&nested_meta)? {
            self.name = name;
        } else if nested_meta.path.is_ident("type") {
            let value = expected_value(&nested_meta)?;
            self.typ = SetTypeConf::from_value(&value)?;
        } else if nested_meta.path.is_ident("full_option") {
            self.full_option = true;
        } else {
            return Err(SynError::new(
                nested_meta.path.span(),
                "this attribute was unknown",
            ));
        }

        Ok(())
    }
}

impl ParseFieldConf for MutFieldConf {
    fn set_option(&mut self, nested_meta: ParseNestedMeta) -> ParseResult<()> {
        if let Some(vis) = VisibilityConf::from_meta(&nested_meta) {
            self.vis = vis;
        } else if let Some(name) = MethodNameConf::from_meta(&nested_meta)? {
            self.name = name;
        } else {
            return Err(SynError::new(
                nested_meta.path.span(),
                "this attribute was unknown",
            ));
        }
        Ok(())
    }
}

impl ParseFieldConf for ClrFieldConf {
    fn set_option(&mut self, nested_meta: ParseNestedMeta) -> ParseResult<()> {
        if let Some(vis) = VisibilityConf::from_meta(&nested_meta) {
            self.vis = vis;
        } else if let Some(name) = MethodNameConf::from_meta(&nested_meta)? {
            self.name = name;
        } else if nested_meta.path.is_ident("scope") {
            let value = expected_value(&nested_meta)?;
            self.scope = ClrScopeConf::from_value(&value)?;
        } else {
            return Err(SynError::new(
                nested_meta.path.span(),
                "this attribute was unknown",
            ));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct FieldConf {
    pub get: GetFieldConf,
    pub set: SetFieldConf,
    pub mut_: MutFieldConf,
    pub clr: ClrFieldConf,
    pub skip: bool,
}

impl ContainerDef {
    pub(crate) fn create(derive_input: syn::DeriveInput) -> ParseResult<Self> {
        let syn::DeriveInput {
            attrs,
            ident,
            generics,
            data,
            ..
        } = derive_input;
        let ident_span = ident.span();
        let syn::Data::Struct(data) = data else {
            return Err(SynError::new(ident_span, "only support structs"));
        };
        let syn::Fields::Named(named_fields) = data.fields else {
            return Err(SynError::new(ident_span, "only support named fields"));
        };
        let conf = ContainerDef::parse_attrs(FieldConf::default(), &attrs[..])?;
        Ok(Self {
            name: ident,
            generics,
            fields: FieldDef::parse_named_fields(named_fields, conf, ident_span)?,
        })
    }
    fn parse_attrs(conf: FieldConf, attrs: &[syn::Attribute]) -> ParseResult<FieldConf> {
        parse_attrs(conf, attrs)
    }
}

impl FieldDef {
    fn parse_named_fields(
        named_fields: syn::FieldsNamed,
        conf: FieldConf,
        span: proc_macro2::Span,
    ) -> ParseResult<Vec<Self>> {
        let mut fields = Vec::new();
        for f in named_fields.named.into_iter() {
            let f_span = f.span();
            let syn::Field {
                attrs, ident, ty, ..
            } = f;
            let conf = FieldDef::parse_attrs(conf.clone(), &attrs[..])?;
            let ident = ident.ok_or_else(|| SynError::new(f_span, "unreachable"))?;
            let field = Self { ident, ty, conf };
            fields.push(field);
        }
        if fields.is_empty() {
            Err(SynError::new(span, "nothing can do for an empty struct"))
        } else {
            Ok(fields)
        }
    }

    fn parse_attrs(conf: FieldConf, attrs: &[syn::Attribute]) -> ParseResult<FieldConf> {
        parse_attrs(conf, attrs)
    }
}

impl GetTypeConf {
    fn from_value(value: &LitStr) -> ParseResult<Self> {
        Ok(match value.value().as_str() {
            "auto" => GetTypeConf::Auto,
            "ref" => GetTypeConf::Ref,
            "copy" => GetTypeConf::Copy,
            "clone" => GetTypeConf::Clone,
            _ => return Err(SynError::new(value.span(), "Unknown `get` type value")),
        })
    }
}

impl SetTypeConf {
    fn from_value(value: &LitStr) -> ParseResult<Self> {
        Ok(match value.value().as_str() {
            "ref" => SetTypeConf::Ref,
            "own" => SetTypeConf::Own,
            "none" => SetTypeConf::None,
            "replace" => SetTypeConf::Replace,
            _ => return Err(SynError::new(value.span(), "Unknown `set` type value")),
        })
    }
}

impl ClrScopeConf {
    fn from_value(value: &LitStr) -> ParseResult<Self> {
        Ok(match value.value().as_str() {
            "auto" => Self::Auto,
            "option" => Self::Option,
            "all" => Self::All,
            _ => return Err(SynError::new(value.span(), "Unknown `clr` scope value")),
        })
    }
}

impl VisibilityConf {
    fn from_meta(meta: &ParseNestedMeta) -> Option<Self> {
        if meta.path.is_ident("disable") {
            Some(VisibilityConf::Disable)
        } else if meta.path.is_ident("public") {
            Some(VisibilityConf::Public)
        } else if meta.path.is_ident("crate") {
            Some(VisibilityConf::Crate)
        } else if meta.path.is_ident("private") {
            Some(VisibilityConf::Private)
        } else {
            None
        }
    }

    pub(crate) fn to_ts(self) -> Option<proc_macro2::TokenStream> {
        match self {
            VisibilityConf::Disable => None,
            VisibilityConf::Public => Some(quote!(pub)),
            VisibilityConf::Crate => Some(quote!(pub(crate))),
            VisibilityConf::Private => Some(quote!()),
        }
    }
}

impl MethodNameConf {
    fn from_meta(meta: &ParseNestedMeta) -> ParseResult<Option<Self>> {
        if meta.path.is_ident("name") {
            let value = expected_value(meta)?;
            Ok(Some(Self::Name(value.value())))
        } else if meta.path.is_ident("prefix") {
            let value = expected_value(meta)?;
            Ok(Some(Self::Format {
                prefix: value.value(),
                suffix: String::new(),
            }))
        } else if meta.path.is_ident("suffix") {
            let value = expected_value(meta)?;
            Ok(Some(Self::Format {
                prefix: String::new(),
                suffix: value.value(),
            }))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn complete(&self, field_name: &syn::Ident) -> syn::Ident {
        let method_name = match self {
            MethodNameConf::Name(name) => name.to_owned(),
            MethodNameConf::Format { prefix, suffix } => {
                format!("{prefix}{field_name}{suffix}")
            }
        };
        syn::Ident::new(&method_name, field_name.span())
    }
}

impl ::std::default::Default for FieldConf {
    fn default() -> Self {
        Self {
            get: GetFieldConf {
                vis: VisibilityConf::Crate,
                name: MethodNameConf::Format {
                    prefix: "".to_owned(),
                    suffix: "".to_owned(),
                },
                typ: GetTypeConf::Auto,
            },
            set: SetFieldConf {
                vis: VisibilityConf::Crate,
                name: MethodNameConf::Format {
                    prefix: "set_".to_owned(),
                    suffix: "".to_owned(),
                },
                typ: SetTypeConf::Ref,
                full_option: false,
            },
            mut_: MutFieldConf {
                vis: VisibilityConf::Disable,
                name: MethodNameConf::Format {
                    prefix: "mut_".to_owned(),
                    suffix: "".to_owned(),
                },
            },
            clr: ClrFieldConf {
                vis: VisibilityConf::Disable,
                name: MethodNameConf::Format {
                    prefix: "clear_".to_owned(),
                    suffix: "".to_owned(),
                },
                scope: ClrScopeConf::Option,
            },
            skip: false,
        }
    }
}

fn parse_attrs(mut conf: FieldConf, attrs: &[syn::Attribute]) -> ParseResult<FieldConf> {
    for attr in attrs.iter() {
        if matches!(attr.style, syn::AttrStyle::Outer) && attr.meta.path().is_ident(ATTR_NAME) {
            let nested = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
            for meta in nested {
                let list = match meta {
                    Meta::Path(path) if path.is_ident(SKIP) => {
                        conf.skip = true;
                        continue;
                    }
                    Meta::List(list) => list,
                    _ => {
                        return Err(SynError::new(
                            meta.span(),
                            "the attribute should not be a list",
                        ))
                    }
                };
                if list.path.is_ident("get") {
                    list.parse_nested_meta(|meta| conf.get.set_option(meta))?;
                } else if list.path.is_ident("set") {
                    list.parse_nested_meta(|meta| conf.set.set_option(meta))?;
                } else if list.path.is_ident("mut_") {
                    list.parse_nested_meta(|meta| conf.mut_.set_option(meta))?;
                } else if list.path.is_ident("clr") {
                    list.parse_nested_meta(|meta| conf.clr.set_option(meta))?;
                } else {
                    return Err(SynError::new(
                        list.path.span(),
                        format!("unsupport attribute `{}`", list.path.into_token_stream()),
                    ));
                }
            }
        }
    }
    Ok(conf)
}

fn expected_value(meta: &ParseNestedMeta) -> ParseResult<LitStr> {
    let value = meta.value()?;
    let value: LitStr = value.parse()?;
    Ok(value)
}
