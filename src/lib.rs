#![doc(html_root_url = "https://docs.rs/prost/0.2.0")]

extern crate bytes;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

mod error;
mod message;
mod types;

#[doc(hidden)]
pub mod encoding;

pub use message::Message;
pub use error::{DecodeError, EncodeError};
