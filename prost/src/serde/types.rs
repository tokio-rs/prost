use alloc::{string::String, vec::Vec};
use core::fmt;

use super::{
    de::CustomDeserialize,
    private::{self, DeserializeInto, _serde},
    ser::CustomSerialize,
    DeserializerConfig, SerializerConfig,
};

impl CustomSerialize for () {
    #[inline]
    fn serialize<S>(&self, serializer: S, _config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        use _serde::ser::SerializeMap;
        serializer.serialize_map(None)?.end()
    }
}

impl<'de> CustomDeserialize<'de> for () {
    #[inline]
    fn deserialize<D>(deserializer: D, _config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> _serde::de::Visitor<'de> for Visitor {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("an empty message")
            }

            #[inline]
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: _serde::de::MapAccess<'de>,
            {
                if map.next_key::<_serde::de::IgnoredAny>()?.is_some() {
                    return Err(<A::Error as _serde::de::Error>::invalid_length(
                        1,
                        &"an empty map",
                    ));
                }
                Ok(())
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}

impl CustomSerialize for bool {
    #[inline]
    fn serialize<S>(&self, serializer: S, _config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        serializer.serialize_bool(*self)
    }
}

impl<'de> CustomDeserialize<'de> for bool {
    #[inline]
    fn deserialize<D>(deserializer: D, config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        private::ForwardDeserializer::deserialize_into(deserializer, config)
    }
}

impl CustomSerialize for i32 {
    #[inline]
    fn serialize<S>(&self, serializer: S, _config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        serializer.serialize_i32(*self)
    }
}

impl<'de> CustomDeserialize<'de> for i32 {
    #[inline]
    fn deserialize<D>(deserializer: D, config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        private::IntDeserializer::deserialize_into(deserializer, config)
    }
}

impl CustomSerialize for u32 {
    #[inline]
    fn serialize<S>(&self, serializer: S, _config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        serializer.serialize_u32(*self)
    }
}

impl<'de> CustomDeserialize<'de> for u32 {
    #[inline]
    fn deserialize<D>(deserializer: D, config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        private::IntDeserializer::deserialize_into(deserializer, config)
    }
}

impl CustomSerialize for i64 {
    #[inline]
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        private::SerAsDisplay(self).serialize(serializer, config)
    }
}

impl<'de> CustomDeserialize<'de> for i64 {
    #[inline]
    fn deserialize<D>(deserializer: D, config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        private::IntDeserializer::deserialize_into(deserializer, config)
    }
}

impl CustomSerialize for u64 {
    #[inline]
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        private::SerAsDisplay(self).serialize(serializer, config)
    }
}

impl<'de> CustomDeserialize<'de> for u64 {
    #[inline]
    fn deserialize<D>(deserializer: D, config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        private::IntDeserializer::deserialize_into(deserializer, config)
    }
}

impl CustomSerialize for str {
    #[inline]
    fn serialize<S>(&self, serializer: S, _config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        serializer.serialize_str(self)
    }
}

impl CustomSerialize for String {
    #[inline]
    fn serialize<S>(&self, serializer: S, _config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        serializer.serialize_str(self)
    }
}

impl<'de> CustomDeserialize<'de> for String {
    #[inline]
    fn deserialize<D>(deserializer: D, config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        private::ForwardDeserializer::deserialize_into(deserializer, config)
    }
}

impl CustomSerialize for Vec<u8> {
    #[inline]
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        private::SerBytesAsBase64(self).serialize(serializer, config)
    }
}

impl<'de> CustomDeserialize<'de> for Vec<u8> {
    #[inline]
    fn deserialize<D>(deserializer: D, config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        private::BytesDeserializer::deserialize_into(deserializer, config)
    }
}

impl CustomSerialize for bytes::Bytes {
    #[inline]
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        private::SerBytesAsBase64(self).serialize(serializer, config)
    }
}

impl<'de> CustomDeserialize<'de> for bytes::Bytes {
    #[inline]
    fn deserialize<D>(deserializer: D, config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        private::BytesDeserializer::deserialize_into(deserializer, config)
    }
}

impl CustomSerialize for f32 {
    #[inline]
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        private::SerFloat32(self).serialize(serializer, config)
    }
}

impl<'de> CustomDeserialize<'de> for f32 {
    #[inline]
    fn deserialize<D>(deserializer: D, config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        private::FloatDeserializer::deserialize_into(deserializer, config)
    }
}

impl CustomSerialize for f64 {
    #[inline]
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        private::SerFloat64(self).serialize(serializer, config)
    }
}

impl<'de> CustomDeserialize<'de> for f64 {
    #[inline]
    fn deserialize<D>(deserializer: D, config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        private::FloatDeserializer::deserialize_into(deserializer, config)
    }
}
