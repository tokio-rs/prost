extern crate byteorder;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

mod message;

#[doc(hidden)]
pub mod scalar;
#[doc(hidden)]
pub mod encodable;

pub use message::Message;
