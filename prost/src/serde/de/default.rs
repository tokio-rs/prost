use core::marker::PhantomData;

use super::{DeserializeInto, DeserializerConfig, MaybeDeserializedValue, OptionDeserializer};

pub struct DefaultDeserializer<W>(PhantomData<W>);

impl<T, W> DeserializeInto<T> for DefaultDeserializer<W>
where
    W: DeserializeInto<T>,
    T: Default,
{
    #[inline]
    fn deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        config: &DeserializerConfig,
    ) -> Result<T, D::Error> {
        let val: Option<T> = OptionDeserializer::<W>::deserialize_into(deserializer, config)?;
        Ok(val.unwrap_or_default())
    }

    fn maybe_deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        config: &DeserializerConfig,
    ) -> Result<MaybeDeserializedValue<T>, D::Error> {
        let val: MaybeDeserializedValue<Option<T>> =
            OptionDeserializer::<W>::maybe_deserialize_into(deserializer, config)?;
        Ok(val.map(|val| val.unwrap_or_default()))
    }
}
