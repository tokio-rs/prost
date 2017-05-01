extern crate byteorder;
extern crate bytes;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

#[cfg(test)]
#[macro_use]
extern crate proto_derive;

/// TODO: remove
#[macro_use]
extern crate log;

mod message;

pub mod encoding;
pub mod field;
pub mod length_delimited;
pub mod numeric;

pub use message::Message;
