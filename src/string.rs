//! String types which wrap `Bytes`/`BytesMut`.
//!
//! `BytesString` and `BytesMutString` are backed by `Bytes` and `BytesMut`
//! respectively, and should be sued in a similar manner.
//!
//! The provided trait impls do not clone (except `FromStr`).
//!
//! UTF8 invariants are checked when strings are created (or bytes are added).

use bytes::{BufMut, Bytes, BytesMut};
use std::borrow::Borrow;
use std::convert::{AsRef, Infallible, TryFrom};
use std::fmt;
use std::ops::Deref;
use std::str::{self, FromStr};

#[derive(Clone, Debug)]
pub enum StringError {
    Utf8ErrorBytes(Bytes),
    Utf8ErrorVec(Vec<u8>),
    BytesMutConversionError(Bytes),
}

impl From<Bytes> for StringError {
    fn from(b: Bytes) -> StringError {
        StringError::BytesMutConversionError(b)
    }
}

#[derive(Clone, Debug, Default, Hash, PartialEq, PartialOrd, Eq, Ord)]
pub struct BytesString {
    bytes: Bytes,
}

impl BytesString {
    #[inline]
    pub fn new() -> BytesString {
        BytesString {
            bytes: Bytes::new(),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bytes.len() == 0
    }

    #[inline]
    pub fn from_str_cloned<S: AsRef<str>>(s: &S) -> BytesString {
        BytesString {
            bytes: s.as_ref().as_bytes().into(),
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.bytes.clear();
    }

    #[inline]
    pub unsafe fn as_bytes_mut(&mut self) -> &mut Bytes {
        &mut self.bytes
    }

    #[inline]
    pub fn try_mut(self) -> Result<BytesMutString, BytesString> {
        match self.bytes.try_mut() {
            Ok(b) => Ok(BytesMutString { bytes: b }),
            Err(b) => Err(BytesString { bytes: b }),
        }
    }

    #[inline]
    unsafe fn from_bytes_unchecked(bytes: Bytes) -> BytesString {
        BytesString { bytes }
    }
}

impl<'a> Into<&'a str> for &'a BytesString {
    fn into(self) -> &'a str {
        // Safe because we establish the utf8 invariants when we move bytes into
        // BytesString.
        unsafe { str::from_utf8_unchecked(self.bytes.as_ref()) }
    }
}

impl Into<Bytes> for BytesString {
    fn into(self) -> Bytes {
        self.bytes
    }
}

impl<'a> Into<&'a [u8]> for &'a BytesString {
    fn into(self) -> &'a [u8] {
        self.bytes.as_ref()
    }
}

impl From<String> for BytesString {
    fn from(s: String) -> BytesString {
        unsafe { Self::from_bytes_unchecked(s.into_bytes().into()) }
    }
}

impl TryFrom<Bytes> for BytesString {
    type Error = StringError;
    fn try_from(b: Bytes) -> Result<BytesString, StringError> {
        if str::from_utf8(b.as_ref()).is_err() {
            return Err(StringError::Utf8ErrorBytes(b));
        }
        unsafe { Ok(Self::from_bytes_unchecked(b)) }
    }
}

impl TryFrom<BytesMut> for BytesString {
    type Error = StringError;
    fn try_from(b: BytesMut) -> Result<BytesString, StringError> {
        if str::from_utf8(b.as_ref()).is_err() {
            return Err(StringError::Utf8ErrorBytes(b.freeze()));
        }
        unsafe { Ok(Self::from_bytes_unchecked(b.freeze())) }
    }
}

impl TryFrom<Vec<u8>> for BytesString {
    type Error = StringError;
    fn try_from(v: Vec<u8>) -> Result<BytesString, StringError> {
        let b = v.into();
        unsafe { Ok(Self::from_bytes_unchecked(b)) }
    }
}

impl FromStr for BytesString {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<BytesString, Infallible> {
        Ok(s.to_owned().into())
    }
}

impl Deref for BytesString {
    type Target = str;
    fn deref(&self) -> &str {
        self.into()
    }
}

impl AsRef<str> for BytesString {
    fn as_ref(&self) -> &str {
        self.into()
    }
}

impl AsRef<[u8]> for BytesString {
    fn as_ref(&self) -> &[u8] {
        self.into()
    }
}

impl Borrow<str> for BytesString {
    fn borrow(&self) -> &str {
        self.into()
    }
}

impl fmt::Display for BytesString {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let s: &str = self.into();
        write!(f, "{}", s)
    }
}

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Eq, Ord)]
pub struct BytesMutString {
    bytes: BytesMut,
}

impl BytesMutString {
    #[inline]
    pub fn new() -> BytesMutString {
        BytesMutString {
            bytes: BytesMut::new(),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    #[inline]
    pub fn freeze(self) -> BytesString {
        BytesString {
            bytes: self.bytes.freeze(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bytes.len() == 0
    }

    #[inline]
    pub fn push(&mut self, c: char) {
        // Adapted from the std implementation of `String::push`
        let len = c.len_utf8();
        // `BytesMut` does not automatically grow its buffer.
        self.reserve(len);
        match len {
            1 => self.bytes.put_u8(c as u8),
            _ => self
                .bytes
                .extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes()),
        }
    }

    #[inline]
    pub fn push_str(&mut self, s: &str) {
        self.bytes.reserve(s.len());
        self.bytes.extend_from_slice(s.as_bytes());
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.bytes.reserve(additional);
    }

    #[inline]
    pub fn from_str_cloned<S: AsRef<str>>(s: &S) -> BytesMutString {
        BytesMutString {
            bytes: s.as_ref().as_bytes().into(),
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.bytes.clear();
    }

    #[inline]
    pub unsafe fn as_bytes_mut(&mut self) -> &mut BytesMut {
        &mut self.bytes
    }

    #[inline]
    unsafe fn from_bytes_unchecked(bytes: BytesMut) -> BytesMutString {
        BytesMutString { bytes }
    }
}

impl<'a> Into<&'a str> for &'a BytesMutString {
    fn into(self) -> &'a str {
        // Safe because we establish the utf8 invariants when we move bytes into
        // BytesMutString.
        unsafe { str::from_utf8_unchecked(self.bytes.as_ref()) }
    }
}

impl Into<Bytes> for BytesMutString {
    fn into(self) -> Bytes {
        self.bytes.freeze()
    }
}

impl Into<BytesMut> for BytesMutString {
    fn into(self) -> BytesMut {
        self.bytes
    }
}

impl<'a> Into<&'a [u8]> for &'a BytesMutString {
    fn into(self) -> &'a [u8] {
        self.bytes.as_ref()
    }
}

impl From<String> for BytesMutString {
    fn from(s: String) -> BytesMutString {
        unsafe { Self::from_bytes_unchecked(s.into_bytes().into()) }
    }
}

impl TryFrom<Bytes> for BytesMutString {
    type Error = StringError;
    fn try_from(b: Bytes) -> Result<BytesMutString, StringError> {
        if str::from_utf8(b.as_ref()).is_err() {
            return Err(StringError::Utf8ErrorBytes(b));
        }
        let b = b.try_mut()?;
        unsafe { Ok(Self::from_bytes_unchecked(b)) }
    }
}

impl TryFrom<BytesMut> for BytesMutString {
    type Error = StringError;
    fn try_from(b: BytesMut) -> Result<BytesMutString, StringError> {
        if str::from_utf8(b.as_ref()).is_err() {
            return Err(StringError::Utf8ErrorBytes(b.freeze()));
        }
        unsafe { Ok(Self::from_bytes_unchecked(b)) }
    }
}

impl TryFrom<Vec<u8>> for BytesMutString {
    type Error = StringError;
    fn try_from(v: Vec<u8>) -> Result<BytesMutString, StringError> {
        let b = v.into();
        unsafe { Ok(Self::from_bytes_unchecked(b)) }
    }
}

impl FromStr for BytesMutString {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<BytesMutString, Infallible> {
        Ok(s.to_owned().into())
    }
}

impl Deref for BytesMutString {
    type Target = str;
    fn deref(&self) -> &str {
        self.into()
    }
}

impl AsRef<str> for BytesMutString {
    fn as_ref(&self) -> &str {
        self.into()
    }
}

impl AsRef<[u8]> for BytesMutString {
    fn as_ref(&self) -> &[u8] {
        self.into()
    }
}

impl Borrow<str> for BytesMutString {
    fn borrow(&self) -> &str {
        self.into()
    }
}

impl fmt::Display for BytesMutString {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let s: &str = self.into();
        write!(f, "{}", s)
    }
}
