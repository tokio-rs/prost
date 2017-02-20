use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::io::{
    Read,
    Result,
    Write,
    Error,
    ErrorKind,
};
use std::usize;

use check_limit;
use field::ScalarField;

/// A protobuf message.
pub trait Message: Any + Debug + Send + Sync {

    /// Write the message to the provided `Write`.
    fn write_to(&self, w: &mut Write) -> Result<()>;

    /// Merge a message of known-size `len` into `self`.
    fn merge_from(&mut self, len: usize, r: &mut Read) -> Result<()>;

    /// Encode the message and its length and write them to the provided `Write`.
    fn write_length_delimited_to(&self, w: &mut Write) -> Result<()> {
        let len = self.wire_len() as u64;
        <u64 as ScalarField>::write_to(&len, w)?;
        self.write_to(w)
    }

    /// Merge a length-delimited message into `self`, the total length may be at most 'limit'
    /// bytes.
    fn merge_length_delimited_from(&mut self, r: &mut Read, limit: &mut usize) -> Result<()> {
        let len = u64::read_from(r, limit)?;
        if len > usize::MAX as u64 {
            return Err(Error::new(ErrorKind::InvalidInput,
                                  "message length overflows usize"));
        }
        check_limit(len as usize, limit)?;
        self.merge_from(len as usize, r)
    }

    /// The encoded length of the message.
    fn wire_len(&self) -> usize;

    fn type_id(&self) -> TypeId;

    fn as_any(&self) -> &Any;

    fn as_any_mut(&mut self) -> &mut Any;

    fn into_any(self: Box<Self>) -> Box<Any>;
}

/// Test that the `Message` trait is object-safe.
#[allow(unused)]
fn test_message_is_object_safe(message: &Message) {}
