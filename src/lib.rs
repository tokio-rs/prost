extern crate byteorder;
extern crate bytes;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

#[cfg(test)]
#[macro_use]
extern crate proto_derive;

mod message;

pub mod encoding;
//pub mod field;

pub use message::Message;
