extern crate byteorder;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

#[macro_use]
extern crate proto_derive;

mod message;

#[doc(hidden)]
pub mod field;

pub use message::Message;

use std::io::{
    Error,
    ErrorKind,
    Result,
};

#[inline]
fn check_limit(needed: usize, limit: &mut usize) -> Result<()> {
    if needed > *limit {
        Err(Error::new(ErrorKind::InvalidData,
                       "read limit exceeded"))
    } else {
        *limit -= needed;
        Ok(())
    }
}
