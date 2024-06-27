//! Dynamic (object-safe) version of std traits.
//! 
//! See: [dyn_derive](https://crates.io/crates/dyn_derive)

pub mod any;
pub mod cmp;
pub mod ops;
pub mod clone;

pub use any::*;
