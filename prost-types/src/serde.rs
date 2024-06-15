use core::{
    collections::BTreeMap,
    fmt::{self, Display},
};

use prost::serde::{
    de::{CustomDeserialize, DesWithConfig},
    private::{self, DeserializeEnum, _serde},
    ser::{CustomSerialize, SerWithConfig},
    DeserializerConfig, SerializerConfig,
};

use crate::{value, Duration, FieldMask, ListValue, NullValue, Struct, Timestamp, Value};

impl CustomSerialize for NullValue {
    #[inline]
    fn serialize<S>(&self, serializer: S, _config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        serializer.serialize_none()
    }
}

impl DeserializeEnum for NullValue {
    #[inline]
    fn deserialize_from_i32<E>(val: i32) -> Result<Result<Self, i32>, E>
    where
        E: _serde::de::Error,
    {
        Err(E::invalid_value(
            _serde::de::Unexpected::Signed(val.into()),
            &"a null value",
        ))
    }

    #[inline]
    fn deserialize_from_str<E>(val: &str) -> Result<Result<Self, i32>, E>
    where
        E: _serde::de::Error,
    {
        Err(E::invalid_value(
            _serde::de::Unexpected::Str(val),
            &"a null value",
        ))
    }

    #[inline]
    fn deserialize_from_null<E>() -> Result<Result<Self, i32>, E>
    where
        E: _serde::de::Error,
    {
        Ok(Ok(Self::NullValue))
    }

    #[inline]
    fn can_deserialize_null() -> bool {
        true
    }
}

impl CustomSerialize for Duration {
    #[inline]
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        if !self.is_valid() {
            return Err(<S::Error as _serde::ser::Error>::custom(format!(
                "duration is invalid: d={:?}",
                self
            )));
        }
        private::SerAsDisplay(self).serialize(serializer, config)
    }
}

impl<'de> CustomDeserialize<'de> for Duration {
    #[inline]
    fn deserialize<D>(deserializer: D, _config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        struct Visitor;

        impl _serde::de::Visitor<'_> for Visitor {
            type Value = Duration;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a duration string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: _serde::de::Error,
            {
                match v.parse::<Duration>() {
                    Ok(val) if val.is_valid() => Ok(val),
                    Ok(_) | Err(_) => Err(E::invalid_value(
                        _serde::de::Unexpected::Str(v),
                        &"a valid duration string",
                    )),
                }
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl CustomSerialize for Timestamp {
    #[inline]
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        private::SerAsDisplay(self).serialize(serializer, config)
    }
}

impl<'de> CustomDeserialize<'de> for Timestamp {
    #[inline]
    fn deserialize<D>(deserializer: D, _config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        struct Visitor;

        impl _serde::de::Visitor<'_> for Visitor {
            type Value = Timestamp;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a timestamp string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: _serde::de::Error,
            {
                Timestamp::from_json_str(v).map_err(|_| {
                    E::invalid_value(_serde::de::Unexpected::Str(v), &"a valid timestamp string")
                })
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl CustomSerialize for FieldMask {
    #[inline]
    fn serialize<S>(&self, serializer: S, _config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        struct FieldMaskAsDisplay<'a>(&'a FieldMask);

        impl Display for FieldMaskAsDisplay<'_> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let mut peek = self.0.paths.iter().peekable();
                while let Some(path) = peek.next() {
                    f.write_str(path)?;
                    if peek.peek().is_some() {
                        f.write_str(",")?;
                    }
                }
                Ok(())
            }
        }

        serializer.collect_str(&FieldMaskAsDisplay(self))
    }
}

impl<'de> CustomDeserialize<'de> for FieldMask {
    #[inline]
    fn deserialize<D>(deserializer: D, _config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        struct Visitor;

        impl _serde::de::Visitor<'_> for Visitor {
            type Value = FieldMask;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a fieldmask string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: _serde::de::Error,
            {
                fn is_valid_ident(ident: &str) -> bool {
                    let mut chars = ident.chars();
                    let Some(first) = chars.next() else {
                        return false;
                    };
                    if !first.is_ascii_alphabetic() {
                        return false;
                    }
                    chars.all(|chr| chr.is_ascii_alphabetic() || chr.is_ascii_digit() || chr == '_')
                }
                let mut paths = vec![];
                let iter = v.split(',').filter_map(|path| {
                    let path = path.trim();
                    (!path.is_empty()).then_some(path)
                });
                for path in iter {
                    if !path.split('.').all(is_valid_ident) {
                        return Err(E::invalid_value(
                            _serde::de::Unexpected::Str(v),
                            &"a valid fieldmask string",
                        ));
                    }
                    paths.push(path.to_owned());
                }
                Ok(FieldMask { paths })
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

impl CustomSerialize for Struct {
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        use _serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(self.fields.len()))?;
        for (key, value) in &self.fields {
            map.serialize_entry(key, &SerWithConfig(value, config))?;
        }
        map.end()
    }
}

impl<'de> CustomDeserialize<'de> for Struct {
    #[inline]
    fn deserialize<D>(deserializer: D, config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        struct Visitor<'c>(&'c DeserializerConfig);

        impl<'c, 'de> _serde::de::Visitor<'de> for Visitor<'c> {
            type Value = Struct;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a Struct")
            }

            #[inline]
            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: _serde::de::MapAccess<'de>,
            {
                deserialize_struct(map, self.0)
            }
        }

        deserializer.deserialize_map(Visitor(config))
    }
}

impl CustomSerialize for crate::protobuf::Any {
    fn serialize<S>(&self, _serializer: S, _config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        panic!("serializing the old prost::Any is not supported")
    }
}

impl<'de> CustomDeserialize<'de> for crate::protobuf::Any {
    #[inline]
    fn deserialize<D>(_deserializer: D, _config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        panic!("deserializing the old prost::Any is not supported")
    }
}

impl CustomSerialize for Value {
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        match self.kind.as_ref() {
            Some(value::Kind::NullValue(_)) | None => serializer.serialize_none(),
            Some(value::Kind::NumberValue(val)) => serializer.serialize_f64(*val),
            Some(value::Kind::StringValue(val)) => serializer.serialize_str(val),
            Some(value::Kind::BoolValue(val)) => serializer.serialize_bool(*val),
            Some(value::Kind::StructValue(val)) => {
                CustomSerialize::serialize(val, serializer, config)
            }
            Some(value::Kind::ListValue(val)) => {
                CustomSerialize::serialize(val, serializer, config)
            }
        }
    }
}

impl<'de> CustomDeserialize<'de> for Value {
    #[inline]
    fn deserialize<D>(deserializer: D, config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        struct Visitor<'c>(&'c DeserializerConfig);

        impl<'c, 'de> _serde::de::Visitor<'de> for Visitor<'c> {
            type Value = Value;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a Value")
            }

            #[inline]
            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: _serde::de::Error,
            {
                Ok(Value {
                    kind: Some(value::Kind::NullValue(0)),
                })
            }

            #[inline]
            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: _serde::de::Error,
            {
                Ok(Value {
                    kind: Some(value::Kind::NullValue(0)),
                })
            }

            #[inline]
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: _serde::de::Error,
            {
                Ok(Value {
                    kind: Some(value::Kind::NumberValue(v as f64)),
                })
            }

            #[inline]
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: _serde::de::Error,
            {
                Ok(Value {
                    kind: Some(value::Kind::NumberValue(v as f64)),
                })
            }

            #[inline]
            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: _serde::de::Error,
            {
                Ok(Value {
                    kind: Some(value::Kind::NumberValue(v)),
                })
            }

            #[inline]
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: _serde::de::Error,
            {
                Ok(Value {
                    kind: Some(value::Kind::StringValue(v.to_owned())),
                })
            }

            #[inline]
            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: _serde::de::Error,
            {
                Ok(Value {
                    kind: Some(value::Kind::BoolValue(v)),
                })
            }

            #[inline]
            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: _serde::de::MapAccess<'de>,
            {
                let value = deserialize_struct(map, self.0)?;
                Ok(Value {
                    kind: Some(value::Kind::StructValue(value)),
                })
            }

            #[inline]
            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: _serde::de::SeqAccess<'de>,
            {
                let value = deserialize_list_value(seq, self.0)?;
                Ok(Value {
                    kind: Some(value::Kind::ListValue(value)),
                })
            }
        }

        deserializer.deserialize_any(Visitor(config))
    }

    #[inline]
    fn can_deserialize_null() -> bool {
        true
    }
}

impl CustomSerialize for ListValue {
    #[inline]
    fn serialize<S>(&self, serializer: S, config: &SerializerConfig) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        serializer.collect_seq(self.values.iter().map(|value| SerWithConfig(value, config)))
    }
}

impl<'de> CustomDeserialize<'de> for ListValue {
    #[inline]
    fn deserialize<D>(deserializer: D, config: &DeserializerConfig) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        struct Visitor<'c>(&'c DeserializerConfig);

        impl<'c, 'de> _serde::de::Visitor<'de> for Visitor<'c> {
            type Value = ListValue;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a ListValue")
            }

            #[inline]
            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: _serde::de::SeqAccess<'de>,
            {
                deserialize_list_value(seq, self.0)
            }
        }

        deserializer.deserialize_seq(Visitor(config))
    }
}

fn deserialize_list_value<'de, A>(
    mut seq: A,
    config: &DeserializerConfig,
) -> Result<ListValue, A::Error>
where
    A: _serde::de::SeqAccess<'de>,
{
    let mut values = vec![];
    while let Some(value) = seq.next_element_seed(DesWithConfig::<Value>::new(config))? {
        values.push(value);
    }
    Ok(ListValue { values })
}

fn deserialize_struct<'de, A>(mut map: A, config: &DeserializerConfig) -> Result<Struct, A::Error>
where
    A: _serde::de::MapAccess<'de>,
{
    let mut fields = BTreeMap::new();
    while let Some(key) = map.next_key::<String>()? {
        let value = map.next_value_seed(DesWithConfig::<Value>::new(config))?;
        fields.insert(key, value);
    }
    Ok(Struct { fields })
}
