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
    fn write_length_delimited_to<W>(&self, w: &mut W) -> Result<()> where W: Write, Self: Sized;

    /// Merge a length-delimited message into `self`.
    fn merge_length_delimited_from<R>(&self, r: &mut R) -> Result<()> where R: Read, Self: Sized;

    /// Like `write_length_delimited_to`, but allows generic `Message` instances
    /// (e.g. `&Message`) to be written.
    fn write_length_delimited_to_dynamic(&self, w: &mut Write) -> Result<()>;

    /// Like `merge_length_delimited_from`, but allows generic `Message` instances
    /// (e.g. `&mut Message`) to be merged.
    fn merge_length_delimited_from_dynamic(&mut self, r: &mut Read) -> Result<()>;

    fn type_id(&self) -> TypeId;

    fn as_any(&self) -> &Any;

    fn as_any_mut(&mut self) -> &mut Any;

    fn into_any(self: Box<Self>) -> Box<Any>;
}

/// Test that the `Message` trait is object-safe.
#[allow(unused)]
fn test_message_is_object_safe(message: &Message) {}
