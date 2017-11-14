#![doc(html_root_url = "https://docs.rs/prost/0.1.1")]

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "alloc", feature(alloc))]

#[cfg(feature = "alloc")]
#[macro_use] extern crate alloc;
#[cfg(feature = "std")]
extern crate core;

extern crate byteorder;
extern crate bytes;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

mod error;
mod message;

#[doc(hidden)]
pub mod encoding;

pub use message::Message;
pub use error::{DecodeError, EncodeError};

/// Custom (internal-only) prelude for this crate.
mod prelude {
    #[cfg(feature = "alloc")]
    pub use alloc::boxed::Box;

    #[cfg(feature = "alloc")]
    pub use alloc::btree_map::BTreeMap;

    #[cfg(feature = "alloc")]
    pub use alloc::borrow::Cow;

    #[cfg(feature = "alloc")]
    pub use alloc::string::String;

    #[cfg(feature = "alloc")]
    pub use alloc::vec::Vec;
}
