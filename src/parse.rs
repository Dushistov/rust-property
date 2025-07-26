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
const NAME_OPTION: (&str, Option<&[&str]>) = ("name", None);
const PREFIX_OPTION: (&str, Option<&[&str]>) = ("prefix", None);
const SUFFIX_OPTION: (&str, Option<&[&str]>) = ("suffix", None);
const VISIBILITY_OPTIONS: &[&str] = &["disable", "public", "crate", "private"];
const GET_TYPE_OPTIONS: (&str, Option<&[&str]>) = ("type", Some(&["auto", "ref", "copy", "clone"]));
const SET_TYPE_OPTIONS: (&str, Option<&[&str]>) =
    ("type", Some(&["ref", "own", "none", "replace"]));
const SET_OPTION_FULL_OPTION: &[&str] = &["full_option"];
const CLR_TYPE_OPTIONS: (&str, Option<&[&str]>) = ("scope", Some(&["auto", "option", "all"]));

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
    pub(crate) fn parse_from_input(
        namevalue_params: &HashMap<&str, String>,
        span: proc_macro2::Span,
    ) -> ParseResult<Option<Self>> {
        let choice = match namevalue_params.get("type").map(AsRef::as_ref) {
            None => None,
            Some("auto") => Some(GetTypeConf::Auto),
            Some("ref") => Some(GetTypeConf::Ref),
            Some("copy") => Some(GetTypeConf::Copy),
            Some("clone") => Some(GetTypeConf::Clone),
            _ => return Err(SynError::new(span, "unreachable result")),
        };
        Ok(choice)
    }
}

impl SetTypeConf {
    pub(crate) fn parse_from_input(
        namevalue_params: &HashMap<&str, String>,
        span: proc_macro2::Span,
    ) -> ParseResult<Option<Self>> {
        let choice = match namevalue_params.get("type").map(AsRef::as_ref) {
            None => None,
            Some("ref") => Some(SetTypeConf::Ref),
            Some("own") => Some(SetTypeConf::Own),
            Some("none") => Some(SetTypeConf::None),
            Some("replace") => Some(SetTypeConf::Replace),
            _ => return Err(SynError::new(span, "unreachable result")),
        };
        Ok(choice)
    }
}

impl ClrScopeConf {
    pub(crate) fn parse_from_input(
        namevalue_params: &HashMap<&str, String>,
        span: proc_macro2::Span,
    ) -> ParseResult<Option<Self>> {
        let choice = match namevalue_params.get("scope").map(AsRef::as_ref) {
            None => None,
            Some("auto") => Some(ClrScopeConf::Auto),
            Some("option") => Some(ClrScopeConf::Option),
            Some("all") => Some(ClrScopeConf::All),
            _ => return Err(SynError::new(span, "unreachable result")),
        };
        Ok(choice)
    }
}

impl VisibilityConf {
    pub(crate) fn parse_from_input(
        input: Option<&str>,
        span: proc_macro2::Span,
    ) -> ParseResult<Option<Self>> {
        let choice = match input {
            None => None,
            Some("disable") => Some(VisibilityConf::Disable),
            Some("public") => Some(VisibilityConf::Public),
            Some("crate") => Some(VisibilityConf::Crate),
            Some("private") => Some(VisibilityConf::Private),
            _ => return Err(SynError::new(span, "unreachable result")),
        };
        Ok(choice)
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
    pub(crate) fn parse_from_input(
        namevalue_params: &HashMap<&str, String>,
        span: proc_macro2::Span,
    ) -> ParseResult<Option<Self>> {
        let name_opt = namevalue_params.get("name").map(ToOwned::to_owned);
        let prefix_opt = namevalue_params.get("prefix").map(ToOwned::to_owned);
        let suffix_opt = namevalue_params.get("suffix").map(ToOwned::to_owned);
        if let Some(name) = name_opt {
            if prefix_opt.is_some() || suffix_opt.is_some() {
                Err(SynError::new(
                    span,
                    "do not set prefix or suffix if name was set",
                ))
            } else {
                Ok(Some(MethodNameConf::Name(name)))
            }
        } else {
            let choice = match (prefix_opt, suffix_opt) {
                (Some(prefix), Some(suffix)) => Some(MethodNameConf::Format { prefix, suffix }),
                (Some(prefix), None) => Some(MethodNameConf::Format {
                    prefix,
                    suffix: "".to_owned(),
                }),
                (None, Some(suffix)) => Some(MethodNameConf::Format {
                    prefix: "".to_owned(),
                    suffix,
                }),
                (None, None) => None,
            };
            Ok(choice)
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
                        let paths = check_path_params(&path_params, &[VISIBILITY_OPTIONS])?;
                        let namevalues = check_namevalue_params(
                            &namevalue_params,
                            &[NAME_OPTION, PREFIX_OPTION, SUFFIX_OPTION, GET_TYPE_OPTIONS],
                        )?;
                        if let Some(choice) =
                            VisibilityConf::parse_from_input(paths[0], list.path.span())?
                        {
                            self.get.vis = choice;
                        }
                        if let Some(choice) =
                            MethodNameConf::parse_from_input(&namevalues, list.path.span())?
                        {
                            self.get.name = choice;
                        }
                        if let Some(choice) =
                            GetTypeConf::parse_from_input(&namevalues, list.path.span())?
                        {
                            self.get.typ = choice;
                        }
                    }
                    "set" => {
                        let paths = check_path_params(
                            &path_params,
                            &[VISIBILITY_OPTIONS, SET_OPTION_FULL_OPTION],
                        )?;
                        let namevalues = check_namevalue_params(
                            &namevalue_params,
                            &[NAME_OPTION, PREFIX_OPTION, SUFFIX_OPTION, SET_TYPE_OPTIONS],
                        )?;
                        if let Some(choice) =
                            VisibilityConf::parse_from_input(paths[0], list.path.span())?
                        {
                            self.set.vis = choice;
                        }
                        self.set.full_option = paths[1].is_some();
                        if let Some(choice) =
                            MethodNameConf::parse_from_input(&namevalues, list.path.span())?
                        {
                            self.set.name = choice;
                        }
                        if let Some(choice) =
                            SetTypeConf::parse_from_input(&namevalues, list.path.span())?
                        {
                            self.set.typ = choice;
                        }
                    }
                    "mut_" => {
                        let paths = check_path_params(&path_params, &[VISIBILITY_OPTIONS])?;
                        let namevalues = check_namevalue_params(
                            &namevalue_params,
                            &[NAME_OPTION, PREFIX_OPTION, SUFFIX_OPTION],
                        )?;
                        if let Some(choice) =
                            VisibilityConf::parse_from_input(paths[0], list.path.span())?
                        {
                            self.mut_.vis = choice;
                        }
                        if let Some(choice) =
                            MethodNameConf::parse_from_input(&namevalues, list.path.span())?
                        {
                            self.mut_.name = choice;
                        }
                    }
                    "clr" => {
                        let paths = check_path_params(&path_params, &[VISIBILITY_OPTIONS])?;
                        let namevalues = check_namevalue_params(
                            &namevalue_params,
                            &[NAME_OPTION, PREFIX_OPTION, SUFFIX_OPTION, CLR_TYPE_OPTIONS],
                        )?;
                        if let Some(choice) =
                            VisibilityConf::parse_from_input(paths[0], list.path.span())?
                        {
                            self.clr.vis = choice;
                        }
                        if let Some(choice) =
                            MethodNameConf::parse_from_input(&namevalues, list.path.span())?
                        {
                            self.clr.name = choice;
                        }
                        if let Some(choice) =
                            ClrScopeConf::parse_from_input(&namevalues, list.path.span())?
                        {
                            self.clr.scope = choice;
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
            syn::Meta::NameValue(name_value) => {
                return Err(SynError::new(
                    name_value.span(),
                    "this attribute should not be a name-value pair",
                ));
            }
        }
        Ok(())
    }
}

fn check_path_params<'a>(
    path_params: &HashSet<&syn::Path>,
    options: &[&[&'a str]],
) -> ParseResult<Vec<Option<&'a str>>> {
    let mut result = vec![None; options.len()];
    let mut find;
    for p in path_params.iter() {
        find = false;
        'outer: for (i, group) in options.iter().enumerate() {
            for opt in group.iter() {
                if p.is_ident(opt) {
                    find = true;
                    if result[i].is_some() {
                        return Err(SynError::new(
                            p.span(),
                            "this kind of attribute has been set twice",
                        ));
                    }
                    result[i] = Some(*opt);
                    break 'outer;
                }
            }
        }
        if !find {
            return Err(SynError::new(p.span(), "this attribute was unknown"));
        }
    }
    Ok(result)
}

fn check_namevalue_params<'a>(
    params: &HashMap<&syn::Path, &syn::LitStr>,
    options: &[(&'a str, Option<&[&'a str]>)],
) -> ParseResult<HashMap<&'a str, String>> {
    let mut result = HashMap::new();
    let mut find;
    for (n, v) in params.iter() {
        find = false;
        let value = v.value();
        'outer: for (k, group_opt) in options.iter() {
            if n.is_ident(k) {
                if let Some(group) = group_opt {
                    for opt in group.iter() {
                        if &value == opt {
                            let _ = result.insert(*k, value.clone());
                            find = true;
                            break 'outer;
                        }
                    }
                } else {
                    let _ = result.insert(*k, value);
                    find = true;
                    break;
                }
            }
        }
        if !find {
            return Err(SynError::new(n.span(), "this attribute was unknown"));
        }
    }
    Ok(result)
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
                syn::Meta::Path(path) => {
                    if path.is_ident(ATTR_NAME) {
                        return Err(SynError::new(
                            path.span(),
                            "the attribute should not be a path",
                        ));
                    }
                }
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
