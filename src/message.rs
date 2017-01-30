use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::io::{
    Read,
    Result,
    Write,
};

/// A protobuf message.
pub trait Message: Any + Debug + Default + Send + Sync {

    /// Encode the message and write it to the provided `Write`.
    fn write_to(&self, w: Write) -> Result<()>;

    /// Encode the message and its length and write them to the provided `Write`.
    fn write_length_delimited_to(&self, w: Write) -> Result<()>;

    fn read_from(&self, r: Read) -> Result<()>;

    fn read_length_delimited_from(&mut self, r: Read) -> Result<()>;

    fn type_id(&self) -> TypeId;

    fn as_any(&self) -> &Any;

    fn as_any_mut(&mut self) -> &mut Any;

    fn into_any(self: Box<Self>) -> Box<Any>;
}
