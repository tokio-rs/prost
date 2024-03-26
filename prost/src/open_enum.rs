use crate::encoding::{DecodeContext, WireType};
use crate::{DecodeError, Message};

use bytes::{Buf, BufMut};

use core::fmt::Debug;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OpenEnum<T> {
    Known(T),
    Unknown(i32),
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
    pub fn from_raw(value: i32) -> Self
    where
        i32: TryInto<T>,
    {
        match value.try_into() {
            Ok(v) => Self::Known(v),
            Err(_) => Self::Unknown(value),
        }
    }

    pub fn into_raw(self) -> i32
    where
        T: Into<i32>,
    {
        match self {
            Self::Known(v) => v.into(),
            Self::Unknown(v) => v,
        }
    }

    pub fn to_raw(&self) -> i32
    where
        T: Clone + Into<i32>,
    {
        match self {
            Self::Known(v) => v.clone().into(),
            Self::Unknown(v) => *v,
        }
    }
}

impl<T> OpenEnum<T> {
    pub fn unwrap(self) -> T {
        match self {
            Self::Known(v) => v,
            Self::Unknown(v) => panic!("unknown field value {}", v),
        }
    }

    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Self::Known(v) => v,
            Self::Unknown(_) => default,
        }
    }

    pub fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce(i32) -> T,
    {
        match self {
            Self::Known(v) => v,
            Self::Unknown(v) => f(v),
        }
    }

    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        match self {
            Self::Known(v) => v,
            Self::Unknown(_) => T::default(),
        }
    }

    pub fn known(self) -> Option<T> {
        match self {
            Self::Known(v) => Some(v),
            Self::Unknown(_) => None,
        }
    }

    pub fn known_or<E>(self, err: E) -> Result<T, E> {
        match self {
            Self::Known(v) => Ok(v),
            Self::Unknown(_) => Err(err),
        }
    }

    pub fn known_or_else<E, F>(self, err: F) -> Result<T, E>
    where
        F: FnOnce(i32) -> E,
    {
        match self {
            Self::Known(v) => Ok(v),
            Self::Unknown(v) => Err(err(v)),
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
