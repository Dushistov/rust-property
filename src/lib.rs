// Copyright (C) 2019-2021 Boyu Yang
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![forbid(unsafe_code)]

use quote::quote;

mod generate;
mod parse;

use crate::{
    generate::derive_property_for_field,
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
