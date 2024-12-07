use alloc::vec::Vec;
use core::{fmt, marker::PhantomData};

use super::{DeserializeInto, DeserializerConfig};

pub struct BytesDeserializer;

impl<T> DeserializeInto<T> for BytesDeserializer
where
    T: From<Vec<u8>>,
{
    #[inline]
    fn deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        _config: &DeserializerConfig,
    ) -> Result<T, D::Error> {
        struct Visitor<T>(PhantomData<T>);

        impl<T> serde::de::Visitor<'_> for Visitor<T>
        where
            T: From<Vec<u8>>,
        {
            type Value = T;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a base64 encoded string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use base64::Engine;

                let err = match base64::prelude::BASE64_STANDARD.decode(v) {
                    Ok(val) => return Ok(T::from(val)),
                    Err(err) => err,
                };
                if let base64::DecodeError::InvalidByte(_, b'-' | b'_') = err {
                    static ENGINE: base64::engine::GeneralPurpose =
                        base64::engine::GeneralPurpose::new(
                            &base64::alphabet::URL_SAFE,
                            base64::engine::GeneralPurposeConfig::new()
                                .with_decode_allow_trailing_bits(true)
                                .with_decode_padding_mode(
                                    base64::engine::DecodePaddingMode::RequireNone,
                                ),
                        );
                    if let Ok(val) = ENGINE.decode(v) {
                        return Ok(T::from(val));
                    }
                }

                Err(E::invalid_value(
                    serde::de::Unexpected::Str(v),
                    &"a valid base64 encoded string",
                ))
            }
        }

        deserializer.deserialize_any(Visitor(PhantomData))
    }
}
