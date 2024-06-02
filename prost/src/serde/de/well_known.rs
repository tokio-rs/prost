use core::marker::PhantomData;

use super::{CustomDeserialize, DeserializeInto, DeserializerConfig};

pub trait UnpackWellKnown {
    type Target;

    fn unpack(self) -> Self::Target;
}

pub struct WellKnownDeserializer<W>(PhantomData<W>);

impl<T, W> DeserializeInto<T> for WellKnownDeserializer<W>
where
    W: for<'de> CustomDeserialize<'de> + UnpackWellKnown<Target = T>,
{
    #[inline]
    fn deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        config: &DeserializerConfig,
    ) -> Result<T, D::Error> {
        Ok(W::deserialize(deserializer, config)?.unpack())
    }

    #[inline]
    fn can_deserialize_null() -> bool {
        W::can_deserialize_null()
    }
}
