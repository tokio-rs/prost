use crate::protobuf::value::Kind;
use crate::protobuf::{ListValue, Struct, Value};
use serde::de::{Error, MapAccess, SeqAccess, Visitor};
use serde::ser::{Error as _, SerializeMap, SerializeSeq};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for Struct {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_map(Some(self.fields.len()))?;
        for (k, v) in &self.fields {
            s.serialize_entry(k, v)?;
        }
        s.end()
    }
}
impl<'de> Deserialize<'de> for Struct {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StructVisitor;

        impl<'de> Visitor<'de> for StructVisitor {
            type Value = Struct;

            fn expecting(&self, formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                formatter.write_str("a map of strings to values")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut fields = ::prost::alloc::collections::BTreeMap::new();
                while let Some((key, value)) = map.next_entry()? {
                    fields.insert(key, value);
                }
                Ok(Struct { fields })
            }
        }

        deserializer.deserialize_map(StructVisitor)
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.kind {
            Some(Kind::NullValue(_)) => serializer.serialize_none(),
            Some(Kind::NumberValue(v)) => serializer.serialize_f64(*v),
            Some(Kind::StringValue(v)) => serializer.serialize_str(v),
            Some(Kind::BoolValue(v)) => serializer.serialize_bool(*v),
            Some(Kind::StructValue(v)) => v.serialize(serializer),
            Some(Kind::ListValue(v)) => v.serialize(serializer),
            None => Err(S::Error::custom("Value kind is None")),
        }
    }
}
impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self {
            kind: Some(Deserialize::deserialize(deserializer)?),
        })
    }
}

impl<'de> Deserialize<'de> for Kind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(KindVisitor)
    }
}

struct KindVisitor;

impl<'de> Visitor<'de> for KindVisitor {
    type Value = Kind;

    fn expecting(&self, formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        formatter.write_str("any valid protobuf value")
    }

    #[inline]
    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Kind::BoolValue(value))
    }

    #[inline]
    fn visit_i64<E: Error>(self, value: i64) -> Result<Self::Value, E> {
        let rounded = value as f64;
        match rounded as i64 == value {
            true => Ok(Kind::NumberValue(value as f64)),
            false => Err(Error::custom("i64 cannot be represented by f64")),
        }
    }

    #[inline]
    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Kind::NumberValue(value as f64))
    }

    #[inline]
    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
        Ok(Kind::NumberValue(value))
    }

    #[inline]
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_string(::prost::alloc::string::String::from(value))
    }

    #[inline]
    fn visit_string<E>(self, value: ::prost::alloc::string::String) -> Result<Self::Value, E> {
        Ok(Kind::StringValue(value))
    }

    #[inline]
    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Kind::NullValue(0))
    }

    #[inline]
    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Kind::NullValue(0))
    }

    #[inline]
    fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut values = ::prost::alloc::vec::Vec::new();

        while let Some(elem) = visitor.next_element()? {
            values.push(elem);
        }

        Ok(Kind::ListValue(ListValue { values }))
    }

    fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut fields = ::prost::alloc::collections::BTreeMap::new();

        while let Some((key, value)) = visitor.next_entry()? {
            fields.insert(key, value);
        }

        Ok(Kind::StructValue(Struct { fields }))
    }
}

impl Serialize for ListValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_seq(Some(self.values.len()))?;
        for v in &self.values {
            s.serialize_element(v)?;
        }
        s.end()
    }
}
impl<'de> Deserialize<'de> for ListValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ListVisitor;

        impl<'de> Visitor<'de> for ListVisitor {
            type Value = ListValue;

            fn expecting(&self, formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                formatter.write_str("a map of strings to values")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut values = ::prost::alloc::vec::Vec::new();
                while let Some((key, value)) = seq.next_element()? {
                    values.insert(key, value);
                }
                Ok(ListValue { values })
            }
        }

        deserializer.deserialize_map(ListVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(value: Value, expected: &str) {
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, expected);

        let value_rt = serde_json::from_str(&json).unwrap();
        assert_eq!(value, value_rt);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_null_value() {
        let value = Value {
            kind: Some(Kind::NullValue(0)),
        };
        round_trip(value, "null");
    }
    #[cfg(feature = "std")]
    #[test]
    fn test_number_value() {
        let value = Value {
            kind: Some(Kind::NumberValue(123.456)),
        };
        round_trip(value, "123.456");
    }
    #[cfg(feature = "std")]
    #[test]
    fn test_string_value() {
        let value = Value {
            kind: Some(Kind::StringValue("test string".into())),
        };
        round_trip(value, "\"test string\"");
    }
    #[cfg(feature = "std")]
    #[test]
    fn test_bool_value() {
        let value = Value {
            kind: Some(Kind::BoolValue(true)),
        };
        round_trip(value, "true");
    }
    #[cfg(feature = "std")]
    #[test]
    fn test_struct_value() {
        let mut fields = ::prost::alloc::collections::BTreeMap::new();
        fields.insert("key1".into(), "value1".into());
        fields.insert("key2".into(), "value2".into());

        let value = Value {
            kind: Some(Kind::StructValue(Struct { fields })),
        };
        round_trip(value, "{\"key1\":\"value1\",\"key2\":\"value2\"}");
    }
    #[cfg(feature = "std")]
    #[test]
    fn test_list_value() {
        let mut values = ::prost::alloc::vec::Vec::new();
        values.push("value1".into());
        values.push("value2".into());

        let value = Value {
            kind: Some(Kind::ListValue(ListValue { values })),
        };
        round_trip(value, "[\"value1\",\"value2\"]");
    }
}
