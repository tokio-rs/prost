use super::{CustomDeserialize, DeserializeInto, DeserializerConfig};

pub struct MessageDeserializer;

impl<T> DeserializeInto<T> for MessageDeserializer
where
    T: for<'de> CustomDeserialize<'de>,
{
    #[inline]
    fn deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        config: &DeserializerConfig,
    ) -> Result<T, D::Error> {
        CustomDeserialize::deserialize(deserializer, config)
    }

    #[inline]
    fn can_deserialize_null() -> bool {
        T::can_deserialize_null()
    }
}
