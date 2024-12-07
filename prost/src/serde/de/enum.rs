use core::{fmt, marker::PhantomData};
use std::borrow::Cow;

use super::{DeserializeInto, DeserializerConfig, MaybeDeserializedValue};

pub trait DeserializeEnum: Sized + Into<i32> {
    fn deserialize_from_i32<E>(val: i32) -> Result<Option<Self>, E>
    where
        E: serde::de::Error;

    fn deserialize_from_str<E>(val: &str) -> Result<Option<Self>, E>
    where
        E: serde::de::Error;

    #[inline]
    fn deserialize_from_null<E>() -> Result<Self, E>
    where
        E: serde::de::Error,
    {
        Err(E::invalid_value(
            serde::de::Unexpected::Option,
            &"a valid enum value",
        ))
    }

    #[inline]
    fn can_deserialize_null() -> bool {
        false
    }
}

pub struct EnumDeserializer<E>(PhantomData<E>);

impl<T> DeserializeInto<i32> for EnumDeserializer<T>
where
    T: DeserializeEnum,
{
    #[inline]
    fn deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        config: &DeserializerConfig,
    ) -> Result<i32, D::Error> {
        match deserializer.deserialize_any(EnumVisitor::<T>(config, PhantomData))? {
            Ok(val) => Ok(val.into()),
            Err(UnknownEnumValue::Int(val)) => Ok(val),
            Err(UnknownEnumValue::Str(val)) => Err(<D::Error as serde::de::Error>::invalid_value(
                serde::de::Unexpected::Str(&val),
                &"a valid enum value",
            )),
        }
    }

    #[inline]
    fn maybe_deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        config: &DeserializerConfig,
    ) -> Result<MaybeDeserializedValue<i32>, D::Error> {
        match deserializer.deserialize_any(EnumVisitor::<T>(config, PhantomData))? {
            Ok(val) => Ok(MaybeDeserializedValue::Val(val.into())),
            Err(UnknownEnumValue::Int(val)) => Ok(MaybeDeserializedValue::Val(val)),
            Err(UnknownEnumValue::Str(_)) => Ok(MaybeDeserializedValue::UnknownEnumValue),
        }
    }

    #[inline]
    fn can_deserialize_null() -> bool {
        T::can_deserialize_null()
    }
}

#[derive(Debug)]
enum UnknownEnumValue<'de> {
    Int(i32),
    Str(Cow<'de, str>),
}

struct EnumVisitor<'c, E>(&'c DeserializerConfig, PhantomData<E>);

impl<'de, T> serde::de::Visitor<'de> for EnumVisitor<'_, T>
where
    T: DeserializeEnum,
{
    type Value = Result<T, UnknownEnumValue<'de>>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an enum")
    }

    #[inline]
    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let val = T::deserialize_from_i32(v)?;
        match val {
            Some(val) => Ok(Ok(val)),
            None if self.0.deny_unknown_enum_values => Err(E::invalid_value(
                serde::de::Unexpected::Signed(v.into()),
                &"a valid enum value",
            )),
            None => Ok(Err(UnknownEnumValue::Int(v))),
        }
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_i32(v.try_into().map_err(|_| {
            E::invalid_value(serde::de::Unexpected::Signed(v), &"a valid enum value")
        })?)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_i32(v.try_into().map_err(|_| {
            E::invalid_value(serde::de::Unexpected::Unsigned(v), &"a valid enum value")
        })?)
    }

    #[inline]
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let val = T::deserialize_from_str(v)?;
        match val {
            Some(val) => Ok(Ok(val)),
            None if self.0.ignore_unknown_enum_string_values => {
                Ok(Err(UnknownEnumValue::Str(Cow::Owned(v.to_owned()))))
            }
            None => Err(E::invalid_value(
                serde::de::Unexpected::Str(v),
                &"a valid enum value",
            )),
        }
    }

    #[inline]
    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let val = T::deserialize_from_str(v)?;
        match val {
            Some(val) => Ok(Ok(val)),
            None if self.0.ignore_unknown_enum_string_values => {
                Ok(Err(UnknownEnumValue::Str(Cow::Borrowed(v))))
            }
            None => Err(E::invalid_value(
                serde::de::Unexpected::Str(v),
                &"a valid enum value",
            )),
        }
    }

    #[inline]
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        T::deserialize_from_null().map(Ok)
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        T::deserialize_from_null().map(Ok)
    }
}
