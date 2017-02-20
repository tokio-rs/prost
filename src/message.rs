use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::io::{
    Read,
    Result,
    Write,
};

/// A protobuf message.
pub trait Message: Any + Debug + Send + Sync {

    /// Encode the message and its length and write them to the provided `Write`.
    fn write_length_delimited_to(&self, w: &mut Write) -> Result<()>;

    /// Merge a length-delimited message into `self`.
    fn merge_length_delimited_from(&mut self, r: &mut Read) -> Result<()>;

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
