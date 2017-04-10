extern crate byteorder;
extern crate bytes;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

mod message;

#[doc(hidden)]
pub mod field;

#[doc(hidden)]
pub mod encoding;

pub use message::Message;

use std::error;
use std::io::{
    Error,
    ErrorKind,
};

fn invalid_data<E>(error: E) -> Error where E: Into<Box<error::Error + Send + Sync>> {
    Error::new(ErrorKind::InvalidData, error.into())
}
fn invalid_input<E>(error: E) -> Error where E: Into<Box<error::Error + Send + Sync>> {
    Error::new(ErrorKind::InvalidInput, error.into())
}

