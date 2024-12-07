use alloc::vec::Vec;

use core::{fmt::Display, marker::PhantomData, ops::Deref};
use serde::{ser::SerializeStruct, Serialize, Serializer};

use super::SerializerConfig;

pub trait CustomSerialize {
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer;
}

impl<T> CustomSerialize for &T
where
    T: CustomSerialize,
{
    #[inline]
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        CustomSerialize::serialize(*self, serializer, config)
    }
}

impl<T> CustomSerialize for [T]
where
    T: CustomSerialize,
{
    #[inline]
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(self.iter().map(|item| SerWithConfig(item, config)))
    }
}

impl<T> CustomSerialize for Vec<T>
where
    T: CustomSerialize,
{
    #[inline]
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        CustomSerialize::serialize(self.as_slice(), serializer, config)
    }
}

impl<T> CustomSerialize for Box<T>
where
    T: CustomSerialize,
{
    #[inline]
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        CustomSerialize::serialize(&**self, serializer, config)
    }
}

pub struct SerWithConfig<'c, T>(pub T, pub &'c SerializerConfig);

impl<T> serde::Serialize for SerWithConfig<'_, T>
where
    T: CustomSerialize,
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        CustomSerialize::serialize(&self.0, serializer, self.1)
    }
}

pub struct SerIdentity<'a, T>(pub &'a T);

impl<T> CustomSerialize for SerIdentity<'_, T>
where
    T: CustomSerialize,
{
    #[inline]
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        CustomSerialize::serialize(self.0, serializer, config)
    }
}

pub struct SerSerde<'a, T>(pub &'a T);

impl<T> CustomSerialize for SerSerde<'_, T>
where
    T: Serialize,
{
    #[inline]
    fn serialize<S>(&self, serializer: S, _config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        T::serialize(self.0, serializer)
    }
}

pub struct SerAsDisplay<'a, T>(pub &'a T);

impl<T> CustomSerialize for SerAsDisplay<'_, T>
where
    T: Display,
{
    #[inline]
    fn serialize<S>(&self, serializer: S, _config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self.0)
    }
}

pub struct SerBytesAsBase64<'a, T>(pub &'a T);

impl<T> CustomSerialize for SerBytesAsBase64<'_, T>
where
    T: Deref<Target = [u8]>,
{
    #[inline]
    fn serialize<S>(&self, serializer: S, _config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&base64::display::Base64Display::new(
            self.0,
            &base64::prelude::BASE64_STANDARD,
        ))
    }
}

pub struct SerFloat32<'a>(pub &'a f32);

impl CustomSerialize for SerFloat32<'_> {
    fn serialize<S>(&self, serializer: S, _config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.0.is_nan() {
            serializer.serialize_str("NaN")
        } else if self.0.is_infinite() {
            if self.0.is_sign_positive() {
                serializer.serialize_str("Infinity")
            } else {
                serializer.serialize_str("-Infinity")
            }
        } else {
            serializer.serialize_f32(*self.0)
        }
    }
}

pub struct SerFloat64<'a>(pub &'a f64);

impl CustomSerialize for SerFloat64<'_> {
    fn serialize<S>(&self, serializer: S, _config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.0.is_nan() {
            serializer.serialize_str("NaN")
        } else if self.0.is_infinite() {
            if self.0.is_sign_positive() {
                serializer.serialize_str("Infinity")
            } else {
                serializer.serialize_str("-Infinity")
            }
        } else {
            serializer.serialize_f64(*self.0)
        }
    }
}

pub struct SerMappedVecItems<'a, I, M>(pub &'a Vec<I>, pub fn(&'a I) -> M);

impl<I, M> CustomSerialize for SerMappedVecItems<'_, I, M>
where
    M: CustomSerialize,
{
    #[inline]
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.0.iter().map(|x| SerWithConfig(self.1(x), config)))
    }
}

pub struct SerEnum<E>(pub i32, PhantomData<E>);

impl<E> SerEnum<E> {
    #[inline]
    pub fn new(val: &i32) -> Self {
        Self(*val, PhantomData)
    }
}

impl<E> CustomSerialize for SerEnum<E>
where
    E: TryFrom<i32> + CustomSerialize,
{
    #[inline]
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Ok(enum_val) = E::try_from(self.0) {
            CustomSerialize::serialize(&enum_val, serializer, config)
        } else {
            serializer.serialize_i32(self.0)
        }
    }
}

pub struct SerMappedMapItems<'a, C, V, M>(pub &'a C, pub fn(&'a V) -> M);

impl<'a, C, K, V, M> CustomSerialize for SerMappedMapItems<'a, C, V, M>
where
    &'a C: IntoIterator<Item = (&'a K, &'a V)>,
    K: Display + 'a,
    M: CustomSerialize,
{
    #[inline]
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_map(self.0.into_iter().map(|(key, val)| {
            (
                SerWithConfig(SerAsDisplay(key), config),
                SerWithConfig(self.1(val), config),
            )
        }))
    }
}

pub trait SerializeOneOf {
    fn serialize_oneof<S>(
        &self,
        serializer: &mut S,
        config: &SerializerConfig,
    ) -> Result<(), S::Error>
    where
        S: SerializeStruct;
}
