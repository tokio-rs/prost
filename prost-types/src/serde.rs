use crate::protobuf::value::Kind;
use crate::protobuf::{ListValue, Struct, Value};
use serde::de::{Error, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq};
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
            None => serializer.serialize_none(),
        }
    }
}
/// Shorthand to create a Value
macro_rules! v {
    ($kind:expr) => {
        Value { kind: Some($kind) }
    };
}
impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                formatter.write_str("any valid protobuf value")
            }

            #[inline]
            fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
                Ok(v!(Kind::BoolValue(value)))
            }

            #[inline]
            fn visit_i64<E: Error>(self, value: i64) -> Result<Value, E> {
                let rounded = value as f64;
                match rounded as i64 == value {
                    true => Ok(v!(Kind::NumberValue(value as f64))),
                    false => Err(Error::custom("i64 cannot be represented by f64")),
                }
            }

            #[inline]
            fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
                Ok(v!(Kind::NumberValue(value as f64)))
            }

            #[inline]
            fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
                Ok(v!(Kind::NumberValue(value)))
            }

            #[cfg(feature = "std")]
            #[inline]
            fn visit_str<E>(self, value: &str) -> Result<Value, E>
            where
                E: Error,
            {
                self.visit_string(::prost::alloc::string::String::from(value))
            }

            #[cfg(feature = "std")]
            #[inline]
            fn visit_string<E>(self, value: ::prost::alloc::string::String) -> Result<Value, E> {
                Ok(v!(Kind::StringValue(value)))
            }

            #[inline]
            fn visit_none<E>(self) -> Result<Value, E> {
                Ok(v!(Kind::NullValue(0)))
            }

            #[inline]
            fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                Deserialize::deserialize(deserializer)
            }

            #[inline]
            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(v!(Kind::NullValue(0)))
            }

            #[inline]
            fn visit_seq<V>(self, mut visitor: V) -> Result<Value, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let mut values = ::prost::alloc::vec::Vec::new();

                while let Some(elem) = visitor.next_element()? {
                    values.push(elem);
                }

                Ok(v!(Kind::ListValue(ListValue { values })))
            }

            #[cfg(feature = "std")]
            fn visit_map<V>(self, mut visitor: V) -> Result<Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut fields = ::prost::alloc::collections::BTreeMap::new();

                while let Some((key, value)) = visitor.next_entry()? {
                    fields.insert(key, value);
                }

                Ok(v!(Kind::StructValue(Struct { fields })))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
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
