# Rust-Property

[![License]](#license)
[![GitHub Actions]](https://github.com/yangby-cryptape/rust-property/actions)
[![Crate Badge]](https://crates.io/crates/property)
[![Crate Doc]](https://docs.rs/property)
[![MSRV 1.87.0]][Rust 1.87.0]

Generate several common methods for structs automatically.

[License]: https://img.shields.io/badge/License-Apache--2.0%20OR%20MIT-blue.svg
[GitHub Actions]: https://github.com/yangby-cryptape/rust-property/workflows/CI/badge.svg
[Crate Badge]: https://img.shields.io/crates/v/property.svg
[Crate Doc]: https://docs.rs/property/badge.svg
[MSRV 1.87.0]: https://img.shields.io/badge/rust-%3E%3D%201.87.0-blue

## Usage

Apply the derive proc-macro `#[derive(Property)]` to structs, and use `#[property(..)]` to configure it.

There are two levels of properties:

- Set container properties can change the default properties for all fields in the container.

- Change the settings of a single field via setting field properties.

If no properties is set, the default properties will be applied:

```rust
#[property(
    get(crate, prefix = "", suffix = "", type="auto"),
    set(crate, prefix = "set_", type = "ref"),
    mut_(disable, prefix = "mut_"),
    clr(disable, prefix = "clear_", scope = "option")
)]
```

There are five kinds of configurable properties: `skip`, `get`, `set`, `mut_` and `clr`.

- If the `skip` property is set, no methods will be generated.

- The visibility of a method can be set via `#[property(get(visibility-type))]`

  There are four kinds of the visibility types: `disable`, `public`, `crate` (default for all methods), and `private`.

- The method name can be set in two ways:

  1. Assign a complete name via `#[property(get(name = "method-name"))]`.

  2. Set `prefix` and / or `suffix` via `#[property(set(prefix = "set_"), mut(suffix = "mut_"))]`.

  The default setting for all fields is: `#[property(get(prefix = "", suffix = ""), set(prefix = "set_"), mut(prefix = "mut_"))]`.

- The return type of `get` method can be set via `#[property(get(type = "return-type"))]`.

  There are four kinds of the return types: `auto` (default), `ref`, `clone` and `copy`.

- The input type and return type of `set` method can be set via `#[property(set(type = "set-type"))]`.

  There are four kinds of the input types: `ref` (default), `own`, `none` and `replace`:

  - `ref`: input is a mutable reference and return is the mutable reference too.

  - `own`: input is a owned object and return is the owned object too.

  - `none`: input is a mutable reference and no return.

  - `replace`: input is a mutable reference and return the old value.

- There is an extra property for `set` method:

  - `full_option`: if the value is `Option<T>`, then the default argument is `T` without this property.

- The `clr` method will set a field to its default value. It has a `scope` property:

  - `auto`: will generate `clr` method for some preset types, such as `Vec`, `Option`, and so on.

  - `option`: (default) will generate `clr` method for `Option` only.

  - `all`: will generate `clr` method for all types.

## In Action

### Original Code

```rust
#![no_std]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std as alloc;

use alloc::{string::String, boxed::Box, vec::Vec};

use property::Property;

#[derive(Copy, Clone)]
pub enum Species {
    Dog,
    Cat,
    Bird,
    Other,
}

#[derive(Property)]
#[property(get(public), clr(scope = "option"), set(private), mut_(disable))]
pub struct Pet {
    #[property(get(name = "identification"), set(disable))]
    id: [u8; 32],
    name: String,
    nickname: Box<str>,
    #[property(set(crate, type = "own"))]
    age: u32,
    #[property(get(type = "copy"))]
    species: Species,
    #[property(get(prefix = "is_"))]
    died: bool,
    #[property(get(type = "clone"), set(type = "none"))]
    owner: String,
    #[property(clr(crate, scope = "auto"))]
    family_members: Vec<String>,
    #[property(get(type = "ref"), mut_(crate))]
    info: String,
    #[property(get(disable), set(type = "replace"))]
    pub tag: Vec<String>,
    #[property(clr(crate), mut_(public, suffix = "_mut"))]
    note: Option<String>,
    #[property(clr(crate), set(type = "replace", full_option))]
    price: Option<u32>,
    #[property(skip)]
    pub reserved: String,
}
```

### Generated Code

```rust
impl Pet {
    #[inline]
    pub fn identification(&self) -> &[u8] {
        &self.id[..]
    }
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[inline]
    fn set_name<T: Into<String>>(&mut self, val: T) -> &mut Self {
        self.name = val.into();
        self
    }
    #[inline]
    pub fn nickname(&self) -> &str {
        &self.nickname
    }
    #[inline]
    fn set_nickname<T: Into<Box<str>>>(&mut self, val: T) -> &mut Self {
        self.nickname = val.into();
        self
    }
    #[inline]
    pub fn age(&self) -> u32 {
        self.age
    }
    #[inline]
    pub(crate) fn set_age<T: Into<u32>>(mut self, val: T) -> Self {
        self.age = val.into();
        self
    }
    #[inline]
    pub fn species(&self) -> Species {
        self.species
    }
    #[inline]
    fn set_species<T: Into<Species>>(&mut self, val: T) -> &mut Self {
        self.species = val.into();
        self
    }
    #[inline]
    pub fn is_died(&self) -> bool {
        self.died
    }
    #[inline]
    fn set_died<T: Into<bool>>(&mut self, val: T) -> &mut Self {
        self.died = val.into();
        self
    }
    #[inline]
    pub fn owner(&self) -> String {
        self.owner.clone()
    }
    #[inline]
    fn set_owner<T: Into<String>>(&mut self, val: T) {
        self.owner = val.into();
    }
    #[inline]
    pub fn family_members(&self) -> &[String] {
        &self.family_members[..]
    }
    #[inline]
    fn set_family_members<T: Into<String>>(
        &mut self,
        val: impl IntoIterator<Item = T>,
    ) -> &mut Self {
        self.family_members = val.into_iter().map(Into::into).collect();
        self
    }
    #[inline]
    pub(crate) fn clear_family_members(&mut self) {
        self.family_members.clear();
    }
    #[inline]
    pub fn info(&self) -> &String {
        &self.info
    }
    #[inline]
    fn set_info<T: Into<String>>(&mut self, val: T) -> &mut Self {
        self.info = val.into();
        self
    }
    #[inline]
    pub(crate) fn mut_info(&mut self) -> &mut String {
        &mut self.info
    }
    #[inline]
    fn set_tag<T: Into<String>>(&mut self, val: impl IntoIterator<Item = T>) -> Vec<String> {
        ::core::mem::replace(&mut self.tag, val.into_iter().map(Into::into).collect())
    }
    #[inline]
    pub fn note(&self) -> Option<&String> {
        self.note.as_ref()
    }
    #[inline]
    fn set_note<T: Into<String>>(&mut self, val: T) -> &mut Self {
        self.note = Some(val.into());
        self
    }
    #[inline]
    pub fn note_mut(&mut self) -> &mut Option<String> {
        &mut self.note
    }
    #[inline]
    pub(crate) fn clear_note(&mut self) {
        self.note = None;
    }
    #[inline]
    pub fn price(&self) -> Option<u32> {
        self.price
    }
    #[inline]
    fn set_price<T: Into<Option<u32>>>(&mut self, val: T) -> Option<u32> {
        ::core::mem::replace(&mut self.price, val.into())
    }
    #[inline]
    pub(crate) fn clear_price(&mut self) {
        self.price = None;
    }
}
```

Enjoy it!

## Minimum Supported Rust Version

[Rust 1.87.0].

## License

Licensed under either of [Apache License, Version 2.0] or [MIT License], at your option.

[Apache License, Version 2.0]: LICENSE-APACHE
[MIT License]: LICENSE-MIT
[Rust 1.87.0]: https://blog.rust-lang.org/2025/05/15/Rust-1.87.0/
