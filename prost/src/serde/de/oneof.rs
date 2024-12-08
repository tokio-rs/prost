use super::DeserializerConfig;

pub trait DeserializeOneOf: Sized {
    type FieldKey;

    fn deserialize_field_key(val: &str) -> Option<Self::FieldKey>;

    fn deserialize_by_field_key<'de, D>(
        field_key: Self::FieldKey,
        deserializer: D,
        config: &DeserializerConfig,
    ) -> Result<Option<Self>, D::Error>
    where
        D: serde::Deserializer<'de>;
}

pub struct OneOfDeserializer<'c, T>(pub T::FieldKey, pub &'c DeserializerConfig)
where
    T: DeserializeOneOf;

impl<'de, T> serde::de::DeserializeSeed<'de> for OneOfDeserializer<'_, T>
where
    T: DeserializeOneOf,
{
    type Value = Option<T>;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize_by_field_key(self.0, deserializer, self.1)
    }
}
