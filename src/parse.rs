// Copyright (C) 2019-2021 Boyu Yang
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use quote::quote;
use std::collections::{HashMap, HashSet};
use syn::{parse::Result as ParseResult, spanned::Spanned, Error as SynError};

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
    fn set_option(&mut self, name: &syn::Path, value: Option<&syn::LitStr>) -> ParseResult<()>;
}

impl ParseFieldConf for GetFieldConf {
    fn set_option(&mut self, name: &syn::Path, value: Option<&syn::LitStr>) -> ParseResult<()> {
        if let Some(vis) = VisibilityConf::from_path(name) {
            self.vis = vis;
            not_expected_value(name, value)?;
        } else if let Some(name) = MethodNameConf::from_name_value(name, value)? {
            self.name = name;
        } else if name.is_ident("type") {
            let value = expected_value(name, value)?;
            self.typ = GetTypeConf::from_value(value)?;
        } else {
            return Err(SynError::new(name.span(), "this attribute was unknown"));
        }

        Ok(())
    }
}

impl ParseFieldConf for SetFieldConf {
    fn set_option(&mut self, name: &syn::Path, value: Option<&syn::LitStr>) -> ParseResult<()> {
        if let Some(vis) = VisibilityConf::from_path(name) {
            self.vis = vis;
            not_expected_value(name, value)?;
        } else if let Some(name) = MethodNameConf::from_name_value(name, value)? {
            self.name = name;
        } else if name.is_ident("type") {
            let value = expected_value(name, value)?;
            self.typ = SetTypeConf::from_value(value)?;
        } else if name.is_ident("full_option") {
            self.full_option = true;
            not_expected_value(name, value)?;
        } else {
            return Err(SynError::new(name.span(), "this attribute was unknown"));
        }

        Ok(())
    }
}

impl ParseFieldConf for MutFieldConf {
    fn set_option(&mut self, name: &syn::Path, value: Option<&syn::LitStr>) -> ParseResult<()> {
        if let Some(vis) = VisibilityConf::from_path(name) {
            self.vis = vis;
            not_expected_value(name, value)?;
        } else if let Some(name) = MethodNameConf::from_name_value(name, value)? {
            self.name = name;
        } else {
            return Err(SynError::new(name.span(), "this attribute was unknown"));
        }
        Ok(())
    }
}

impl ParseFieldConf for ClrFieldConf {
    fn set_option(&mut self, name: &syn::Path, value: Option<&syn::LitStr>) -> ParseResult<()> {
        if let Some(vis) = VisibilityConf::from_path(name) {
            self.vis = vis;
            not_expected_value(name, value)?;
        } else if let Some(name) = MethodNameConf::from_name_value(name, value)? {
            self.name = name;
        } else if name.is_ident("scope") {
            let value = expected_value(name, value)?;
            self.scope = ClrScopeConf::from_value(value)?;
        } else {
            return Err(SynError::new(name.span(), "this attribute was unknown"));
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
        let attrs_span = derive_input.span();
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
        let conf = ContainerDef::parse_attrs(attrs_span, FieldConf::default(), &attrs[..])?;
        Ok(Self {
            name: ident,
            generics,
            fields: FieldDef::parse_named_fields(named_fields, conf, ident_span)?,
        })
    }
    fn parse_attrs(
        span: proc_macro2::Span,
        conf: FieldConf,
        attrs: &[syn::Attribute],
    ) -> ParseResult<FieldConf> {
        parse_attrs(span, conf, attrs)
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
            let conf = FieldDef::parse_attrs(f_span, conf.clone(), &attrs[..])?;
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

    fn parse_attrs(
        span: proc_macro2::Span,
        conf: FieldConf,
        attrs: &[syn::Attribute],
    ) -> ParseResult<FieldConf> {
        parse_attrs(span, conf, attrs)
    }
}

impl GetTypeConf {
    fn from_value(value: &syn::LitStr) -> ParseResult<Self> {
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
    fn from_value(value: &syn::LitStr) -> ParseResult<Self> {
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
    fn from_value(value: &syn::LitStr) -> ParseResult<Self> {
        Ok(match value.value().as_str() {
            "auto" => Self::Auto,
            "option" => Self::Option,
            "all" => Self::All,
            _ => return Err(SynError::new(value.span(), "Unknown `clr` scope value")),
        })
    }
}

impl VisibilityConf {
    fn from_path(name: &syn::Path) -> Option<Self> {
        if name.is_ident("disable") {
            Some(VisibilityConf::Disable)
        } else if name.is_ident("public") {
            Some(VisibilityConf::Public)
        } else if name.is_ident("crate") {
            Some(VisibilityConf::Crate)
        } else if name.is_ident("private") {
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
    fn from_name_value(name: &syn::Path, value: Option<&syn::LitStr>) -> ParseResult<Option<Self>> {
        if name.is_ident("name") {
            let value = expected_value(name, value)?;
            Ok(Some(Self::Name(value.value())))
        } else if name.is_ident("prefix") {
            let value = expected_value(name, value)?;
            Ok(Some(Self::Format {
                prefix: value.value(),
                suffix: String::new(),
            }))
        } else if name.is_ident("suffix") {
            let value = expected_value(name, value)?;
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

impl FieldConf {
    fn apply_attrs(&mut self, meta: &syn::Meta) -> ParseResult<()> {
        match meta {
            syn::Meta::NameValue(name_value) => {
                return Err(SynError::new(
                    name_value.span(),
                    "this attribute should not be a name-value pair",
                ));
            }
            syn::Meta::Path(path) => {
                if path.is_ident(SKIP) {
                    self.skip = true;
                } else {
                    return Err(SynError::new(path.span(), "this attribute was unknown"));
                }
            }
            syn::Meta::List(list) => {
                let mut path_params = HashSet::new();
                let mut namevalue_params = HashMap::new();
                for nested_meta in list.nested.iter() {
                    match nested_meta {
                        syn::NestedMeta::Meta(meta) => match meta {
                            syn::Meta::Path(path) => {
                                if !path_params.insert(path) {
                                    return Err(SynError::new(
                                        path.span(),
                                        "this attribute has been set twice",
                                    ));
                                }
                            }
                            syn::Meta::NameValue(mnv) => {
                                let syn::MetaNameValue { path, lit, .. } = mnv;
                                let syn::Lit::Str(content) = lit else {
                                    return Err(SynError::new(
                                        lit.span(),
                                        "this literal should be a string literal",
                                    ));
                                };
                                if namevalue_params.insert(path, content).is_some() {
                                    return Err(SynError::new(
                                        path.span(),
                                        "this attribute has been set twice",
                                    ));
                                }
                            }
                            _ => {
                                return Err(SynError::new(
                                    meta.span(),
                                    "this attribute should be a path or a name-value pair",
                                ));
                            }
                        },
                        syn::NestedMeta::Lit(lit) => {
                            return Err(SynError::new(
                                lit.span(),
                                "this attribute should not be a literal",
                            ));
                        }
                    }
                }
                if path_params.is_empty() && namevalue_params.is_empty() {
                    return Err(SynError::new(
                        list.span(),
                        "this attribute should not be empty",
                    ));
                }
                match list
                    .path
                    .get_ident()
                    .ok_or_else(|| {
                        SynError::new(list.path.span(), "this attribute should be a single ident")
                    })?
                    .to_string()
                    .as_ref()
                {
                    "get" => {
                        for p in &path_params {
                            self.get.set_option(p, None)?;
                        }
                        for (k, v) in &namevalue_params {
                            self.get.set_option(k, Some(v))?;
                        }
                    }
                    "set" => {
                        for p in &path_params {
                            self.set.set_option(p, None)?;
                        }
                        for (k, v) in &namevalue_params {
                            self.set.set_option(k, Some(v))?;
                        }
                    }
                    "mut_" => {
                        for p in &path_params {
                            self.mut_.set_option(p, None)?;
                        }
                        for (k, v) in &namevalue_params {
                            self.mut_.set_option(k, Some(v))?;
                        }
                    }
                    "clr" => {
                        for p in &path_params {
                            self.clr.set_option(p, None)?;
                        }
                        for (k, v) in &namevalue_params {
                            self.clr.set_option(k, Some(v))?;
                        }
                    }
                    attr => {
                        return Err(SynError::new(
                            list.path.span(),
                            format!("unsupport attribute `{attr}`"),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

fn parse_attrs(
    span: proc_macro2::Span,
    mut conf: FieldConf,
    attrs: &[syn::Attribute],
) -> ParseResult<FieldConf> {
    for attr in attrs.iter() {
        if let syn::AttrStyle::Outer = attr.style {
            let meta = attr
                .parse_meta()
                .map_err(|_| SynError::new(span, "failed to parse the attributes"))?;
            match meta {
                syn::Meta::List(list) => {
                    if list.path.is_ident(ATTR_NAME) {
                        if list.nested.is_empty() {
                            return Err(SynError::new(
                                list.span(),
                                "this attribute should not be empty",
                            ));
                        }
                        for nested_meta in list.nested.iter() {
                            let syn::NestedMeta::Meta(meta) = nested_meta else {
                                return Err(SynError::new(
                                    nested_meta.span(),
                                    "the attribute in nested meta should be a list",
                                ));
                            };
                            conf.apply_attrs(meta)?;
                        }
                    }
                }
                syn::Meta::Path(path) => {
                    if path.is_ident(ATTR_NAME) {
                        return Err(SynError::new(
                            path.span(),
                            "the attribute should not be a path",
                        ));
                    }
                }
                syn::Meta::NameValue(name_value) => {
                    if name_value.path.is_ident(ATTR_NAME) {
                        return Err(SynError::new(
                            name_value.span(),
                            "the attribute should not be a name-value pair",
                        ));
                    }
                }
            }
        }
    }
    Ok(conf)
}

fn not_expected_value(name: &syn::Path, value: Option<&syn::LitStr>) -> ParseResult<()> {
    if value.is_some() {
        Err(SynError::new(
            name.span(),
            "not expected value for this attribute",
        ))
    } else {
        Ok(())
    }
}

fn expected_value<'a>(
    name: &syn::Path,
    value: Option<&'a syn::LitStr>,
) -> ParseResult<&'a syn::LitStr> {
    value.ok_or_else(|| {
        SynError::new(
            name.span(),
            "Expect string literal value for this attribute",
        )
    })
}
