#![feature(rustc_private)]

// #[macro_use]
// extern crate failure;
extern crate serde;
extern crate prost_types;

pub mod ser;
pub mod de;
pub mod error;

pub use de::{Deserializer};
// pub use error::{Error, Result};
pub use ser::{to_string, Serializer};