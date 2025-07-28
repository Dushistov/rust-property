// Copyright (C) 2019-2021 Boyu Yang
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use quote::quote;
use syn::{punctuated::Punctuated, token::Comma, GenericArgument};

use crate::{ClrScopeConf, FieldDef, GetTypeConf, SetTypeConf};

pub(crate) enum GetType {
    Ref,
    Copy,
    Clone,
    String,
    Slice(syn::TypeSlice),
    Option(Punctuated<GenericArgument, Comma>),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClrMethod {
    SetZero,
    SetNone,
    SetDefault,
    CallClear,
    FillWithDefault,
    None,
}

pub(crate) enum FieldType {
    Number,
    Boolean,
    Character,
    String,
    Box(Punctuated<GenericArgument, Comma>),
    Array(syn::TypeArray),
    Vector(syn::Type),
    Option(Punctuated<GenericArgument, Comma>),
    Unhandled(Option<String>),
}

impl GetType {
    pub(crate) fn from_field_type(ty: &FieldType) -> Self {
        match ty {
            FieldType::Number | FieldType::Boolean | FieldType::Character => GetType::Copy,
            FieldType::String => GetType::String,
            FieldType::Array(type_array) => {
                let syn::TypeArray {
                    bracket_token,
                    elem,
                    ..
                } = type_array.clone();
                GetType::Slice(syn::TypeSlice {
                    bracket_token,
                    elem,
                })
            }
            FieldType::Vector(inner_type) => GetType::Slice(syn::TypeSlice {
                bracket_token: syn::token::Bracket::default(),
                elem: Box::new(inner_type.clone()),
            }),
            FieldType::Box(_) => GetType::Ref,
            FieldType::Option(inner_type) => {
                if inner_type.len() == 1 {
                    if let Some(syn::GenericArgument::Type(inner_type)) = inner_type.first() {
                        if let GetType::Copy =
                            GetType::from_field_type(&FieldType::from_type(inner_type))
                        {
                            return GetType::Copy;
                        }
                    }
                }
                GetType::Option(inner_type.clone())
            }
            FieldType::Unhandled(_) => GetType::Ref,
        }
    }
}

impl ClrMethod {
    pub(crate) fn from_field_type(ty: &FieldType) -> Self {
        match ty {
            FieldType::Number => ClrMethod::SetZero,
            FieldType::Option(_) => ClrMethod::SetNone,
            FieldType::Boolean | FieldType::Character => ClrMethod::SetDefault,
            FieldType::String | FieldType::Vector(_) => ClrMethod::CallClear,
            FieldType::Array(_) => ClrMethod::FillWithDefault,
            FieldType::Unhandled(Some(type_name)) => match type_name.as_str() {
                "String" | "PathBuf" | "Vec" | "VecDeque" | "LinkedList" | "HashMap"
                | "BTreeMap" | "HashSet" | "BTreeSet" | "BinaryHeap" => ClrMethod::CallClear,
                _ => ClrMethod::None,
            },
            _ => ClrMethod::None,
        }
    }
}

impl FieldType {
    pub(crate) fn from_type(ty: &syn::Type) -> Self {
        match ty {
            syn::Type::Path(type_path) => {
                let segs = &type_path.path.segments;
                if !segs.is_empty() {
                    match segs[0].ident.to_string().as_ref() {
                        "f32" | "f64" => FieldType::Number,
                        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => FieldType::Number,
                        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => FieldType::Number,
                        "bool" => FieldType::Boolean,
                        "char" => FieldType::Character,
                        "String" => FieldType::String,
                        "Vec" => {
                            if let syn::PathArguments::AngleBracketed(inner) =
                                &type_path.path.segments[0].arguments
                            {
                                if let syn::GenericArgument::Type(ref inner_type) = inner.args[0] {
                                    FieldType::Vector(inner_type.clone())
                                } else {
                                    unreachable!()
                                }
                            } else {
                                unreachable!()
                            }
                        }
                        "Box" => {
                            let syn::PathArguments::AngleBracketed(inner) =
                                &type_path.path.segments[0].arguments
                            else {
                                unreachable!()
                            };
                            FieldType::Box(inner.args.clone())
                        }
                        "Option" => {
                            if let syn::PathArguments::AngleBracketed(inner) =
                                &type_path.path.segments[0].arguments
                            {
                                FieldType::Option(inner.args.clone())
                            } else {
                                unreachable!()
                            }
                        }
                        _ => {
                            let type_name = segs.last().cloned().unwrap().ident.to_string();
                            FieldType::Unhandled(Some(type_name))
                        }
                    }
                } else {
                    FieldType::Unhandled(None)
                }
            }
            syn::Type::Array(type_array) => FieldType::Array(type_array.clone()),
            _ => FieldType::Unhandled(None),
        }
    }
}

pub(crate) fn derive_property_for_field(field: &FieldDef) -> Vec<proc_macro2::TokenStream> {
    let mut property = Vec::new();
    let field_type = &field.ty;
    let field_name = &field.ident;
    let field_conf = &field.conf;
    let prop_field_type = FieldType::from_type(field_type);
    if let Some(ts) = field_conf.get.vis.to_ts().map(|visibility| {
        let method_name = field_conf.get.name.complete(field_name);
        let get_type = match field_conf.get.typ {
            GetTypeConf::Auto => GetType::from_field_type(&prop_field_type),
            GetTypeConf::Ref => GetType::Ref,
            GetTypeConf::Copy => GetType::Copy,
            GetTypeConf::Clone => GetType::Clone,
        };
        let mut field_type = field_type;
        if let FieldType::Box(ref boxed_ty) = prop_field_type {
            if boxed_ty.len() == 1 && matches!(field_conf.get.typ, GetTypeConf::Auto) {
                if let Some(syn::GenericArgument::Type(inner_type)) = boxed_ty.first() {
                    if *inner_type == syn::parse_quote! { str } {
                        field_type = inner_type;
                    }
                }
            }
        }
        match get_type {
            GetType::Ref => quote!(
                #visibility fn #method_name(&self) -> &#field_type {
                    &self.#field_name
                }
            ),
            GetType::Copy => quote!(
                #visibility fn #method_name(&self) -> #field_type {
                    self.#field_name
                }
            ),
            GetType::Clone => quote!(
                #visibility fn #method_name(&self) -> #field_type {
                    self.#field_name.clone()
                }
            ),
            GetType::String => quote!(
                #visibility fn #method_name(&self) -> &str {
                    &self.#field_name[..]
                }
            ),
            GetType::Slice(field_type) => quote!(
                #visibility fn #method_name(&self) -> &#field_type {
                    &self.#field_name[..]
                }
            ),
            GetType::Option(field_type) => quote!(
                #visibility fn #method_name(&self) -> Option<&#field_type> {
                    self.#field_name.as_ref()
                }
            ),
        }
    }) {
        property.push(ts);
    }
    if let Some(ts) = field_conf.set.vis.to_ts().map(|visibility| {
        let method_name = field_conf.set.name.complete(field_name);
        match &prop_field_type {
            FieldType::Vector(inner_type) => match field_conf.set.typ {
                SetTypeConf::Ref => quote!(
                    #visibility fn #method_name<T: Into<#inner_type>>(
                       &mut self,
                       val: impl IntoIterator<Item = T>
                    ) -> &mut Self {
                        self.#field_name = val.into_iter().map(Into::into).collect();
                        self
                    }
                ),
                SetTypeConf::Own => quote!(
                    #visibility fn #method_name<T: Into<#inner_type>>(
                        mut self,
                        val: impl IntoIterator<Item = T>
                    ) -> Self {
                        self.#field_name = val.into_iter().map(Into::into).collect();
                        self
                    }
                ),
                SetTypeConf::None => quote!(
                    #visibility fn #method_name<T: Into<#inner_type>>(
                       &mut self,
                       val: impl IntoIterator<Item = T>
                    ) {
                        self.#field_name = val.into_iter().map(Into::into).collect();
                    }
                ),
                SetTypeConf::Replace => quote!(
                    #visibility fn #method_name<T: Into<#inner_type>>(
                       &mut self,
                       val: impl IntoIterator<Item = T>
                    ) -> #field_type {
                        ::core::mem::replace(&mut self.#field_name, val.into_iter().map(Into::into).collect())
                    }
                ),
            },
            FieldType::Option(inner_type) if !field_conf.set.full_option => match field_conf.set.typ {
                SetTypeConf::Ref => quote!(
                    #visibility fn #method_name<T: Into<#inner_type>>(
                        &mut self, val: T
                    ) -> &mut Self {
                        self.#field_name = Some(val.into());
                        self
                    }
                ),
                SetTypeConf::Own => quote!(
                    #visibility fn #method_name<T: Into<#inner_type>>(
                        mut self, val: T
                    ) -> Self {
                        self.#field_name = Some(val.into());
                        self
                    }
                ),
                SetTypeConf::None => quote!(
                    #visibility fn #method_name<T: Into<#inner_type>>(
                        &mut self, val: T
                    ) {
                        self.#field_name = Some(val.into());
                    }
                ),
                SetTypeConf::Replace => quote!(
                    #visibility fn #method_name<T: Into<#inner_type>>(
                        &mut self, val: T
                    ) -> #field_type {
                        self.#field_name.replace(val.into())
                    }
                ),
            },
            _ => match field_conf.set.typ {
                SetTypeConf::Ref => quote!(
                    #visibility fn #method_name<T: Into<#field_type>>(
                        &mut self, val: T
                    ) -> &mut Self {
                        self.#field_name = val.into();
                        self
                    }
                ),
                SetTypeConf::Own => quote!(
                    #visibility fn #method_name<T: Into<#field_type>>(
                        mut self, val: T
                    ) -> Self {
                        self.#field_name = val.into();
                        self
                    }
                ),
                SetTypeConf::None => quote!(
                    #visibility fn #method_name<T: Into<#field_type>>(
                        &mut self, val: T
                    ) {
                        self.#field_name = val.into();
                    }
                ),
                SetTypeConf::Replace => quote!(
                    #visibility fn #method_name<T: Into<#field_type>>(
                        &mut self, val: T
                    ) -> #field_type {
                        ::core::mem::replace(&mut self.#field_name, val.into())
                    }
                ),
            },
        }
    }) {
        property.push(ts);
    }
    if let Some(ts) = field_conf.mut_.vis.to_ts().map(|visibility| {
        let method_name = field_conf.mut_.name.complete(field_name);
        quote!(
            #visibility fn #method_name(&mut self) -> &mut #field_type {
                &mut self.#field_name
            }
        )
    }) {
        property.push(ts);
    }
    if let Some(ts) = field_conf.clr.vis.to_ts().and_then(|visibility| {
        let method_name = field_conf.clr.name.complete(field_name);
        let auto_clr_method = ClrMethod::from_field_type(&prop_field_type);
        let clr_method = match field_conf.clr.scope {
            ClrScopeConf::Auto => auto_clr_method,
            ClrScopeConf::Option => {
                if auto_clr_method == ClrMethod::SetNone {
                    auto_clr_method
                } else {
                    ClrMethod::None
                }
            }
            ClrScopeConf::All => {
                if auto_clr_method == ClrMethod::None {
                    ClrMethod::SetDefault
                } else {
                    auto_clr_method
                }
            }
        };
        match clr_method {
            ClrMethod::SetZero => Some(quote!(
                #visibility fn #method_name(&mut self) {
                    self.#field_name = 0;
                }
            )),
            ClrMethod::SetNone => Some(quote!(
                #visibility fn #method_name(&mut self) {
                    self.#field_name =None;
                }
            )),
            ClrMethod::SetDefault => Some(quote!(
                #visibility fn #method_name(&mut self) {
                    self.#field_name = Default::default();
                }
            )),
            ClrMethod::CallClear => Some(quote!(
                #visibility fn #method_name(&mut self) {
                    self.#field_name.clear();
                }
            )),
            ClrMethod::FillWithDefault => Some(quote!(
                #visibility fn #method_name(&mut self) {
                    self.#field_name.fill_with(Default::default);
                }
            )),
            ClrMethod::None => None,
        }
    }) {
        property.push(ts);
    }
    property
}
