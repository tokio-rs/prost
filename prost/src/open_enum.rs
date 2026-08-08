use crate::encoding::{DecodeContext, WireType};
use crate::{DecodeError, Message, UnknownEnumValue};

use bytes::{Buf, BufMut};

use core::fmt::{self, Debug};
use core::hash::{Hash, Hasher};

/// Represents the value of an open enum field.
///
/// The [Protocol Buffers guide][proto-guide] specifies that unknown values
/// of fields with open enum types should be stored directly in the field
/// when decoding messages. This type provides an ergonomic way to represent
/// such values in Rust.
///
/// [proto-guide]: https://protobuf.dev/programming-guides/enum/#definitions
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OpenEnum<T> {
    /// A known value of the generated enum type.
    Known(T),
    /// An unknown value as decoded from the message.
    Unknown(Unknown),
}

impl<T> Default for OpenEnum<T>
where
    T: Default,
{
    fn default() -> Self {
        Self::Known(T::default())
    }
}

impl<T> From<T> for OpenEnum<T> {
    fn from(value: T) -> Self {
        Self::Known(value)
    }
}

impl<T> OpenEnum<T> {
    /// Converts a raw integer value into an open enum value.
    ///
    /// This method is used to decode field values from the wire format.
    pub fn from_raw(value: i32) -> Self
    where
        i32: TryInto<T>,
    {
        match value.try_into() {
            Ok(v) => Self::Known(v),
            Err(_) => Self::Unknown(Unknown(value)),
        }
    }

    /// Converts an open enum value into its raw integer representation.
    pub fn into_raw(self) -> i32
    where
        T: Into<i32>,
    {
        match self {
            Self::Known(v) => v.into(),
            Self::Unknown(u) => u.0,
        }
    }

    /// Converts an open enum value into its raw integer representation.
    ///
    /// This is a convenience method for borrowed values.
    pub fn to_raw(&self) -> i32
    where
        T: Clone + Into<i32>,
    {
        match self {
            Self::Known(v) => v.clone().into(),
            Self::Unknown(u) => u.0,
        }
    }

    /// Returns the known value of the open enum.
    ///
    /// # Panics
    ///
    /// Panics if the value is in fact unknown.
    pub fn unwrap(self) -> T {
        match self {
            Self::Known(v) => v,
            Self::Unknown(u) => panic!("unknown enumeration value {}", u.0),
        }
    }

    /// Returns the known value of the open enum, or, if the value is unknown,
    /// returns the provided default value.
    ///
    /// Arguments passed to `unwrap_or` are eagerly evaluated; if you are passing
    /// the result of a function call, it is recommended to use
    /// [`unwrap_or_else`] and pass a lazily evaluated closure to it.
    ///
    /// [`unwrap_or_else`]: #method.unwrap_or_else
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Self::Known(v) => v,
            Self::Unknown(_) => default,
        }
    }

    /// Returns the known value of the open enum, or, if the value is unknown,
    /// returns the value computed from the field's raw integer value by the
    /// provided closure.
    pub fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce(i32) -> T,
    {
        match self {
            Self::Known(v) => v,
            Self::Unknown(u) => f(u.0),
        }
    }

    /// Returns the known value of the open enum, or, if the value is unknown,
    /// returns the default value of the enum type.
    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        match self {
            Self::Known(v) => v,
            Self::Unknown(_) => T::default(),
        }
    }

    /// If the value of the open enum is known, returns it in `Ok`, otherwise
    /// returns an `Err` with the unknown value.
    pub fn get(&self) -> Result<T, UnknownEnumValue>
    where
        T: Clone,
    {
        match self {
            Self::Known(v) => Ok(v.clone()),
            Self::Unknown(u) => Err(UnknownEnumValue(u.0)),
        }
    }

    /// Sets the value of receiver to the provided known value.
    pub fn set(&mut self, value: T) {
        *self = Self::Known(value);
    }

    /// If the value of the open enum is known, returns it in `Some`, otherwise
    /// returns `None`.
    pub fn known(self) -> Option<T> {
        match self {
            Self::Known(v) => Some(v),
            Self::Unknown(_) => None,
        }
    }

    /// If the value of the open enum is known, returns it in `Ok`, otherwise
    /// returns the provided error value in `Err`.
    ///
    /// Arguments passed to `ok_or` are eagerly evaluated; if you are passing
    /// the result of a function call, it is recommended to use
    /// [`ok_or_else`] and pass a lazily evaluated closure to it.
    ///
    /// [`ok_or_else`]: #method.ok_or_else
    pub fn ok_or<E>(self, err: E) -> Result<T, E> {
        match self {
            Self::Known(v) => Ok(v),
            Self::Unknown(_) => Err(err),
        }
    }

    /// If the value of the open enum is known, returns it in `Ok`, otherwise
    /// returns `Err` with the value computed from the field's raw integer value
    /// by the provided closure.
    pub fn ok_or_else<E, F>(self, err: F) -> Result<T, E>
    where
        F: FnOnce(i32) -> E,
    {
        match self {
            Self::Known(v) => Ok(v),
            Self::Unknown(u) => Err(err(u.0)),
        }
    }
}

impl<T: Hash> Hash for OpenEnum<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            OpenEnum::Known(v) => v.hash(state),
            OpenEnum::Unknown(u) => u.0.hash(state),
        }
    }
}

impl<T> Message for OpenEnum<T>
where
    T: Clone + Into<i32> + Debug + Send + Sync,
    i32: TryInto<T>,
{
    fn encoded_len(&self) -> usize {
        self.to_raw().encoded_len()
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        self.to_raw().encode_raw(buf)
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        let mut raw = 0;
        <i32 as Message>::merge_field(&mut raw, tag, wire_type, buf, ctx)?;
        *self = OpenEnum::from_raw(raw);
        Ok(())
    }

    fn clear(&mut self) {
        *self = OpenEnum::from_raw(0);
    }
}

/// Represents an unknown enumeration value.
///
/// When the Protobuf spec mandates that enumeration value sets are ‘open’,
/// a value of this type represents an integer value not known from the
/// presently used enum definition.
///
/// This wrapper type is used to ensure correctness of constructed `[OpenEnum]`
/// values and should rarely be used by name.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Unknown(i32);

impl Debug for Unknown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<Unknown> for i32 {
    fn from(value: Unknown) -> Self {
        value.0
    }
}

impl From<Unknown> for UnknownEnumValue {
    fn from(value: Unknown) -> Self {
        UnknownEnumValue(value.0)
    }
}
