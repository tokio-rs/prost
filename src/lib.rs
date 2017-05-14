extern crate byteorder;
extern crate bytes;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

mod message;

pub mod encoding;

pub use message::Message;
