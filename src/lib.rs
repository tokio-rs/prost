extern crate byteorder;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

mod message;

pub mod scalar;

pub use message::Message;
