use super::{DeserializeInto, DeserializerConfig};

pub struct ForwardDeserializer;

impl<T> DeserializeInto<T> for ForwardDeserializer
where
    T: for<'de> serde::Deserialize<'de>,
{
    #[inline]
    fn deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        _config: &DeserializerConfig,
    ) -> Result<T, D::Error> {
        <T as serde::Deserialize>::deserialize(deserializer)
    }
}
