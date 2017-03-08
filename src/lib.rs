extern crate byteorder;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

mod message;

#[doc(hidden)]
pub mod field;

#[doc(hidden)]
pub mod field_ng;

pub use message::Message;

use std::io::{
    Error,
    ErrorKind,
    Result,
};

#[inline]
fn check_limit(needed: usize, limit: &mut usize) -> Result<()> {
    if needed > *limit {
        Err(Error::new(ErrorKind::InvalidData, "read limit exceeded"))
    } else {
        *limit -= needed;
        Ok(())
    }
}
