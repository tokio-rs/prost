#![doc(html_root_url = "https://docs.rs/prost/0.1.1")]

extern crate bytes;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

mod error;
mod message;

#[doc(hidden)]
pub mod encoding;

pub mod types;

pub use message::Message;
pub use error::{DecodeError, EncodeError};
