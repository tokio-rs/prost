use core::fmt;

use super::{DeserializeInto, DeserializerConfig};

pub struct BoolDeserializer<const PARSE_STR: bool>;

impl<const PARSE_STR: bool> DeserializeInto<bool> for BoolDeserializer<PARSE_STR> {
    #[inline]
    fn deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        _config: &DeserializerConfig,
    ) -> Result<bool, D::Error> {
        struct Visitor<const PARSE_STR: bool>;

        impl<const PARSE_STR: bool> serde::de::Visitor<'_> for Visitor<PARSE_STR> {
            type Value = bool;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a boolean value")
            }

            #[inline]
            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v)
            }

            #[inline]
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if PARSE_STR {
                    match v {
                        "true" => return Ok(true),
                        "false" => return Ok(false),
                        _ => (),
                    }
                }
                Err(E::invalid_type(
                    serde::de::Unexpected::Str(v),
                    &"a valid boolean value",
                ))
            }
        }

        deserializer.deserialize_any(Visitor::<PARSE_STR>)
    }
}

pub struct IntDeserializer;

impl DeserializeInto<i32> for IntDeserializer {
    #[inline]
    fn deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        _config: &DeserializerConfig,
    ) -> Result<i32, D::Error> {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = i32;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a numeric value (i32)")
            }

            #[inline]
            fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v as i32)
            }

            #[inline]
            fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v as i32)
            }

            #[inline]
            fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v)
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.try_into().map_err(|_| {
                    E::invalid_value(serde::de::Unexpected::Signed(v), &"a valid integer (i32)")
                })
            }

            #[inline]
            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v as i32)
            }

            #[inline]
            fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v as i32)
            }

            #[inline]
            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_u64(v as u64)
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.try_into().map_err(|_| {
                    E::invalid_value(serde::de::Unexpected::Unsigned(v), &"a valid integer (i32)")
                })
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let conv = v as i32;
                if conv as f64 == v {
                    Ok(conv)
                } else {
                    Err(E::invalid_value(
                        serde::de::Unexpected::Float(v),
                        &"a valid integer (i32)",
                    ))
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse::<i32>().map_err(|_| {
                    E::invalid_value(serde::de::Unexpected::Str(v), &"a valid integer (i32)")
                })
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl DeserializeInto<i64> for IntDeserializer {
    #[inline]
    fn deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        _config: &DeserializerConfig,
    ) -> Result<i64, D::Error> {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = i64;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a numeric value (i64)")
            }

            #[inline]
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v)
            }

            #[inline]
            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v as i64)
            }

            #[inline]
            fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v as i64)
            }

            #[inline]
            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v as i64)
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.try_into().map_err(|_| {
                    E::invalid_value(serde::de::Unexpected::Unsigned(v), &"a valid integer (i64)")
                })
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let conv = v as i64;
                if conv as f64 == v {
                    Ok(conv)
                } else {
                    Err(E::invalid_value(
                        serde::de::Unexpected::Float(v),
                        &"a valid integer (i64)",
                    ))
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse::<i64>().map_err(|_| {
                    E::invalid_value(serde::de::Unexpected::Str(v), &"a valid integer (i64)")
                })
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl DeserializeInto<u32> for IntDeserializer {
    #[inline]
    fn deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        _config: &DeserializerConfig,
    ) -> Result<u32, D::Error> {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = u32;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a numeric value (u32)")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.try_into().map_err(|_| {
                    E::invalid_value(serde::de::Unexpected::Signed(v), &"a valid integer (u32)")
                })
            }

            #[inline]
            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v as u32)
            }

            #[inline]
            fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v as u32)
            }

            #[inline]
            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v)
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.try_into().map_err(|_| {
                    E::invalid_value(serde::de::Unexpected::Unsigned(v), &"a valid integer (u32)")
                })
            }

            #[inline]
            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let conv = v as u32;
                if conv as f64 == v {
                    Ok(conv)
                } else {
                    Err(E::invalid_value(
                        serde::de::Unexpected::Float(v),
                        &"a valid integer (u32)",
                    ))
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse::<u32>().map_err(|_| {
                    E::invalid_value(serde::de::Unexpected::Str(v), &"a valid integer (u32)")
                })
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl DeserializeInto<u64> for IntDeserializer {
    #[inline]
    fn deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        _config: &DeserializerConfig,
    ) -> Result<u64, D::Error> {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = u64;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a numeric value (u64)")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.try_into().map_err(|_| {
                    E::invalid_value(serde::de::Unexpected::Signed(v), &"a valid integer (u64)")
                })
            }

            #[inline]
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v)
            }

            #[inline]
            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let conv = v as u64;
                if conv as f64 == v {
                    Ok(conv)
                } else {
                    Err(E::invalid_value(
                        serde::de::Unexpected::Float(v),
                        &"a valid integer (u64)",
                    ))
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse::<u64>().map_err(|_| {
                    E::invalid_value(serde::de::Unexpected::Str(v), &"a valid integer (u32)")
                })
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

pub struct FloatDeserializer;

impl DeserializeInto<f32> for FloatDeserializer {
    #[inline]
    fn deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        _config: &DeserializerConfig,
    ) -> Result<f32, D::Error> {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = f32;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a float (f32)")
            }

            #[inline]
            fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v)
            }

            #[inline]
            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let conv = v as f32;
                if conv.is_finite() {
                    Ok(v as f32)
                } else {
                    Err(E::invalid_value(
                        serde::de::Unexpected::Float(v),
                        &"a floating point number (f32)",
                    ))
                }
            }

            #[inline]
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let conv = v as f32;
                if conv as i64 == v {
                    Ok(conv)
                } else {
                    Err(E::invalid_value(
                        serde::de::Unexpected::Signed(v),
                        &"a floating point number (f32)",
                    ))
                }
            }

            #[inline]
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let conv = v as f32;
                if conv as u64 == v {
                    Ok(conv)
                } else {
                    Err(E::invalid_value(
                        serde::de::Unexpected::Unsigned(v),
                        &"a floating point number (f32)",
                    ))
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    "NaN" => Ok(f32::NAN),
                    "Infinity" => Ok(f32::INFINITY),
                    "-Infinity" => Ok(f32::NEG_INFINITY),
                    v => match v.parse::<f32>() {
                        Ok(v) if !v.is_infinite() => Ok(v),
                        _ => Err(E::invalid_value(
                            serde::de::Unexpected::Str(v),
                            &"a floating point number (f32)",
                        )),
                    },
                }
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl DeserializeInto<f64> for FloatDeserializer {
    #[inline]
    fn deserialize_into<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
        _config: &DeserializerConfig,
    ) -> Result<f64, D::Error> {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = f64;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a float (f64)")
            }

            #[inline]
            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v)
            }

            #[inline]
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let conv = v as f64;
                if conv as i64 == v {
                    Ok(conv)
                } else {
                    Err(E::invalid_value(
                        serde::de::Unexpected::Signed(v),
                        &"a floating point number (f64)",
                    ))
                }
            }

            #[inline]
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let conv = v as f64;
                if conv as u64 == v {
                    Ok(conv)
                } else {
                    Err(E::invalid_value(
                        serde::de::Unexpected::Unsigned(v),
                        &"a floating point number (f64)",
                    ))
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    "NaN" => Ok(f64::NAN),
                    "Infinity" => Ok(f64::INFINITY),
                    "-Infinity" => Ok(f64::NEG_INFINITY),
                    v => match v.parse::<f64>() {
                        Ok(v) if !v.is_infinite() => Ok(v),
                        _ => Err(E::invalid_value(
                            serde::de::Unexpected::Str(v),
                            &"a floating point number (f64)",
                        )),
                    },
                }
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}
