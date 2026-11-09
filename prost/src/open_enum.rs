use crate::encoding::{DecodeContext, WireType};
use crate::{DecodeError, Message, UnknownEnumValue};

use bytes::{Buf, BufMut};

use core::cmp::Ordering;
use core::fmt::{self, Debug};
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::mem::size_of;
use core::slice;

/// Represents the value of an open enum field, which per the
/// [Protocol Buffers guide][proto-guide] must retain values not known to the
/// schema in use.
///
/// This is a `#[repr(transparent)]` wrapper around the `i32` wire value, so it
/// costs the same as the `i32` it replaces and every `i32` is valid. Use
/// [`known`](Self::known) to project it into `T` for exhaustive matching, or
/// compare it against a variant of `T` directly.
///
/// [proto-guide]: https://protobuf.dev/programming-guides/enum/#definitions
#[repr(transparent)]
pub struct OpenEnum<T> {
    raw: i32,
    // `fn() -> T` rather than `T` keeps the auto traits and the impls below
    // unconditional in `T`, without unsafe impls.
    _marker: PhantomData<fn() -> T>,
}

const _: () = assert!(size_of::<OpenEnum<()>>() == size_of::<i32>());

impl<T> OpenEnum<T> {
    /// Converts a raw integer value from the wire into an open enum value.
    ///
    /// Every `i32` is representable, so this never fails.
    pub const fn from_raw(value: i32) -> Self {
        Self {
            raw: value,
            _marker: PhantomData,
        }
    }

    /// Converts an open enum value into its raw integer representation.
    pub const fn into_raw(self) -> i32 {
        self.raw
    }

    /// Returns the raw integer representation of a borrowed value.
    pub const fn to_raw(&self) -> i32 {
        self.raw
    }

    /// Reinterprets raw integer values as open enum values, for free.
    pub const fn from_raw_slice(values: &[i32]) -> &[Self] {
        // SAFETY: `OpenEnum<T>` is `#[repr(transparent)]` over `i32`, so the
        // two slice types have identical layout, and every bit pattern of a
        // valid `i32` is a valid `OpenEnum<T>`.
        unsafe { slice::from_raw_parts(values.as_ptr().cast(), values.len()) }
    }

    /// Reinterprets open enum values as raw integer values, for free.
    pub const fn as_raw_slice(values: &[Self]) -> &[i32] {
        // SAFETY: see `from_raw_slice`.
        unsafe { slice::from_raw_parts(values.as_ptr().cast(), values.len()) }
    }
}

impl<T> OpenEnum<T>
where
    i32: TryInto<T>,
{
    /// Returns the value in `Some` if it is known, otherwise `None`.
    ///
    /// Matching on the result is exhaustiveness checked over `T`.
    pub fn known(self) -> Option<T> {
        self.raw.try_into().ok()
    }

    /// Returns `true` if the value is a known variant of `T`.
    pub fn is_known(self) -> bool {
        self.raw.try_into().is_ok()
    }

    /// Returns the known value.
    ///
    /// # Panics
    ///
    /// Panics if the value is unknown.
    pub fn unwrap(self) -> T {
        match self.raw.try_into() {
            Ok(value) => value,
            Err(_) => panic!("unknown enumeration value {}", self.raw),
        }
    }

    /// Returns the known value, or `default` if it is unknown.
    ///
    /// `default` is eagerly evaluated; see [`unwrap_or_else`](Self::unwrap_or_else).
    pub fn unwrap_or(self, default: T) -> T {
        self.raw.try_into().unwrap_or(default)
    }

    /// Returns the known value, or one computed from the raw value if unknown.
    pub fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce(i32) -> T,
    {
        match self.raw.try_into() {
            Ok(value) => value,
            Err(_) => f(self.raw),
        }
    }

    /// Returns the known value, or `T::default()` if it is unknown.
    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        self.raw.try_into().unwrap_or_default()
    }

    /// Returns the value in `Ok` if it is known, otherwise the raw value in `Err`.
    pub fn get(&self) -> Result<T, UnknownEnumValue> {
        self.raw.try_into().map_err(|_| UnknownEnumValue(self.raw))
    }

    /// Returns the value in `Ok` if it is known, otherwise `err`.
    ///
    /// `err` is eagerly evaluated; see [`ok_or_else`](Self::ok_or_else).
    pub fn ok_or<E>(self, err: E) -> Result<T, E> {
        self.raw.try_into().map_err(|_| err)
    }

    /// Returns the value in `Ok` if it is known, otherwise `err` applied to the
    /// raw value.
    pub fn ok_or_else<E, F>(self, err: F) -> Result<T, E>
    where
        F: FnOnce(i32) -> E,
    {
        self.raw.try_into().map_err(|_| err(self.raw))
    }
}

impl<T> OpenEnum<T>
where
    T: Into<i32>,
{
    /// Sets the receiver to a known value.
    pub fn set(&mut self, value: T) {
        self.raw = value.into();
    }
}

impl<T> Copy for OpenEnum<T> {}

impl<T> Clone for OpenEnum<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Default for OpenEnum<T>
where
    T: Default + Into<i32>,
{
    fn default() -> Self {
        Self::from_raw(T::default().into())
    }
}

impl<T> From<T> for OpenEnum<T>
where
    T: Into<i32>,
{
    fn from(value: T) -> Self {
        Self::from_raw(value.into())
    }
}

// The comparison and hashing impls operate on the raw value. This makes them
// available for every `T`, and agrees with the derived impls of the generated
// `#[repr(i32)]` enum types.

impl<T> PartialEq for OpenEnum<T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<T> Eq for OpenEnum<T> {}

impl<T> PartialOrd for OpenEnum<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for OpenEnum<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.raw.cmp(&other.raw)
    }
}

impl<T> Hash for OpenEnum<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

/// Allows `msg.field == Enum::Variant` without a conversion.
impl<T> PartialEq<T> for OpenEnum<T>
where
    T: Copy + Into<i32>,
{
    fn eq(&self, other: &T) -> bool {
        self.raw == (*other).into()
    }
}

impl<T> Debug for OpenEnum<T>
where
    T: Debug,
    i32: TryInto<T>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.raw.try_into() {
            Ok(value) => f.debug_tuple("Known").field(&value).finish(),
            Err(_) => f.debug_tuple("Unknown").field(&self.raw).finish(),
        }
    }
}

impl<T> Message for OpenEnum<T> {
    fn encoded_len(&self) -> usize {
        self.raw.encoded_len()
    }

    fn encode_raw(&self, buf: &mut impl BufMut) {
        self.raw.encode_raw(buf)
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut impl Buf,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        <i32 as Message>::merge_field(&mut self.raw, tag, wire_type, buf, ctx)
    }

    fn clear(&mut self) {
        self.raw = 0;
    }
}
