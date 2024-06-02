use core::{fmt, marker::PhantomData};

use super::{DeserializeInto, DeserializerConfig};

pub trait DeserializeEnum: Sized + Into<i32> {
    fn deserialize_from_i32<E>(val: i32) -> Result<Result<Self, i32>, E>
    where
        E: serde::de::Error;

    fn deserialize_from_str<E>(val: &str) -> Result<Result<Self, i32>, E>
    where
        E: serde::de::Error;

    #[inline]
    fn deserialize_from_null<E>() -> Result<Result<Self, i32>, E>
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
        struct Visitor<'c, E>(&'c DeserializerConfig, PhantomData<E>);

        impl<'c, 'de, T> serde::de::Visitor<'de> for Visitor<'c, T>
        where
            T: DeserializeEnum,
        {
            type Value = Result<T, i32>;

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
                    Ok(val) => Ok(Ok(val)),
                    Err(raw) if self.0.deny_unknown_enum_values => Err(E::invalid_value(
                        serde::de::Unexpected::Signed(raw.into()),
                        &"a valid enum value",
                    )),
                    Err(raw) => Ok(Err(raw)),
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
                T::deserialize_from_str(v)
            }

            #[inline]
            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                T::deserialize_from_null()
            }

            #[inline]
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                T::deserialize_from_null()
            }
        }

        match deserializer.deserialize_any(Visitor::<T>(config, PhantomData))? {
            Ok(val) => Ok(val.into()),
            Err(val) => Ok(val),
        }
    }

    #[inline]
    fn can_deserialize_null() -> bool {
        T::can_deserialize_null()
    }
}
