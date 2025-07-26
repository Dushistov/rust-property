// Copyright (C) 2019-2021 Boyu Yang
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![forbid(unsafe_code)]

extern crate proc_macro;

use quote::quote;

mod generate;
mod parse;

use crate::{
    generate::{ClrMethod, FieldType, GetType},
    parse::{ClrScopeConf, ContainerDef, FieldDef, GetTypeConf, SetTypeConf},
};

/// Generate several common methods for structs automatically.
#[proc_macro_derive(Property, attributes(property))]
pub fn derive_property(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let property = syn::parse_macro_input!(input as ContainerDef);
    let expanded = {
        let name = &property.name;
        let (impl_generics, type_generics, where_clause_opt) = property.generics.split_for_impl();
        let methods = property.fields.iter().fold(Vec::new(), |mut r, f| {
            if !f.conf.skip {
                r.append(&mut derive_property_for_field(f));
            }
            r
        });
        let impl_methods = quote!(
            impl #impl_generics #name #type_generics #where_clause_opt {
                #(#[inline] #methods)*
            }
        );
        impl_methods
    };
    expanded.into()
}

fn derive_property_for_field(field: &FieldDef) -> Vec<proc_macro2::TokenStream> {
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
            GetTypeConf::Copy_ => GetType::Copy_,
            GetTypeConf::Clone_ => GetType::Clone_,
        };
        match get_type {
            GetType::Ref => quote!(
                #visibility fn #method_name(&self) -> &#field_type {
                    &self.#field_name
                }
            ),
            GetType::Copy_ => quote!(
                #visibility fn #method_name(&self) -> #field_type {
                    self.#field_name
                }
            ),
            GetType::Clone_ => quote!(
                #visibility fn #method_name(&self) -> #field_type {
                    self.#field_name.clone()
                }
            ),
            GetType::String_ => quote!(
                #visibility fn #method_name(&self) -> &str {
                    &self.#field_name[..]
                }
            ),
            GetType::Slice(field_type) => quote!(
                #visibility fn #method_name(&self) -> &#field_type {
                    &self.#field_name[..]
                }
            ),
            GetType::Option_(field_type) => quote!(
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
                SetTypeConf::None_ => quote!(
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
            FieldType::Option_(inner_type) if !field_conf.set.full_option => match field_conf.set.typ {
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
                SetTypeConf::None_ => quote!(
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
                SetTypeConf::None_ => quote!(
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
            ClrScopeConf::Option_ => {
                if auto_clr_method == ClrMethod::SetNone {
                    auto_clr_method
                } else {
                    ClrMethod::None_
                }
            }
            ClrScopeConf::All => {
                if auto_clr_method == ClrMethod::None_ {
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
            ClrMethod::None_ => None,
        }
    }) {
        property.push(ts);
    }
    property
}
