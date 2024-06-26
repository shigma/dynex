#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;

mod derive;
mod dyn_impl;

/// This derive macro has the exact same behavior as `PartialEq`,
/// but it workarounds a strange behavior of the Rust compiler.
/// 
/// For other traits, you can just derive the original trait name.
/// 
/// ## Example
/// 
/// ```
/// use dynex::*;
/// 
/// #[dyn_trait]
/// pub trait Meta: Clone + PartialEq {}
/// 
/// #[derive(Clone, PartialEqFix)]
/// pub struct Foo {
///     meta: Box<dyn Meta>,
/// }
/// ```
#[proc_macro_derive(PartialEqFix)]
pub fn derive_partial_eq(input: TokenStream) -> TokenStream {
    derive::partial_eq::derive(input.into()).into()
}

/// This is a procedural macro for deriving object-unsafe traits.
/// 
/// ## Example
/// 
/// `Clone` is not object-safe, but with this macro, you can still use `dyn Meta`:
/// 
/// ```
/// use dynex::*;
/// 
/// #[dyn_trait]
/// pub trait Meta: Clone {}
/// 
/// #[derive(Clone)]
/// pub struct Foo {
///     meta: Box<dyn Meta>,
/// }
/// ```
#[proc_macro_attribute]
pub fn dyn_trait(attrs: TokenStream, input: TokenStream) -> TokenStream {
    if !attrs.is_empty() {
        panic!("dyn_impl attribute does not accept any arguments")
    }
    dyn_impl::main(input.into()).into()
}
