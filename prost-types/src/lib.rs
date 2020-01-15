#![doc(html_root_url = "https://docs.rs/prost-types/0.6.0")]

//! Protocol Buffers well-known types.
//!
//! Note that the documentation for the types defined in this crate are generated from the Protobuf
//! definitions, so code examples are not in Rust.
//!
//! See the [Protobuf reference][1] for more information about well-known types.
//!
//! [1]: https://developers.google.com/protocol-buffers/docs/reference/google.protobuf

use std::borrow::Cow;
use std::convert::TryFrom;
use std::i32;
use std::i64;
use std::time;

use chrono::prelude::*;

use std::fmt;
use serde::ser::{Serialize, Serializer, SerializeSeq, SerializeMap, SerializeStruct};
use serde::de::{self, Deserialize, Deserializer, Visitor, SeqAccess, MapAccess};
use std::collections::BTreeMap;
use std::str::FromStr;

include!("protobuf.rs");
pub mod compiler {
    include!("compiler.rs");
}

// The Protobuf `Duration` and `Timestamp` types can't delegate to the standard library equivalents
// because the Protobuf versions are signed. To make them easier to work with, `From` conversions
// are defined in both directions.

const NANOS_PER_SECOND: i32 = 1_000_000_000;

impl Duration {
    /// Normalizes the duration to a canonical format.
    ///
    /// Based on [`google::protobuf::util::CreateNormalized`][1].
    /// [1]: https://github.com/google/protobuf/blob/v3.3.2/src/google/protobuf/util/time_util.cc#L79-L100
    fn normalize(&mut self) {
        // Make sure nanos is in the range.
        if self.nanos <= -NANOS_PER_SECOND || self.nanos >= NANOS_PER_SECOND {
            self.seconds += (self.nanos / NANOS_PER_SECOND) as i64;
            self.nanos %= NANOS_PER_SECOND;
        }

        // nanos should have the same sign as seconds.
        if self.seconds < 0 && self.nanos > 0 {
            self.seconds += 1;
            self.nanos -= NANOS_PER_SECOND;
        } else if self.seconds > 0 && self.nanos < 0 {
            self.seconds -= 1;
            self.nanos += NANOS_PER_SECOND;
        }
        // TODO: should this be checked?
        // debug_assert!(self.seconds >= -315_576_000_000 && self.seconds <= 315_576_000_000,
        //               "invalid duration: {:?}", self);
    }
}

/// Converts a `std::time::Duration` to a `Duration`.
impl From<time::Duration> for Duration {
    fn from(duration: time::Duration) -> Duration {
        let seconds = duration.as_secs();
        let seconds = if seconds > i64::MAX as u64 {
            i64::MAX
        } else {
            seconds as i64
        };
        let nanos = duration.subsec_nanos();
        let nanos = if nanos > i32::MAX as u32 {
            i32::MAX
        } else {
            nanos as i32
        };
        let mut duration = Duration { seconds, nanos };
        duration.normalize();
        duration
    }
}

impl TryFrom<Duration> for time::Duration {
    type Error = time::Duration;

    /// Converts a `Duration` to a result containing a positive (`Ok`) or negative (`Err`)
    /// `std::time::Duration`.
    fn try_from(mut duration: Duration) -> Result<time::Duration, time::Duration> {
        duration.normalize();
        if duration.seconds >= 0 {
            Ok(time::Duration::new(
                duration.seconds as u64,
                duration.nanos as u32,
            ))
        } else {
            Err(time::Duration::new(
                (-duration.seconds) as u64,
                (-duration.nanos) as u32,
            ))
        }
    }
}

impl Timestamp {
    /// Normalizes the timestamp to a canonical format.
    ///
    /// Based on [`google::protobuf::util::CreateNormalized`][1].
    /// [1]: https://github.com/google/protobuf/blob/v3.3.2/src/google/protobuf/util/time_util.cc#L59-L77
    fn normalize(&mut self) {
        // Make sure nanos is in the range.
        if self.nanos <= -NANOS_PER_SECOND || self.nanos >= NANOS_PER_SECOND {
            self.seconds += (self.nanos / NANOS_PER_SECOND) as i64;
            self.nanos %= NANOS_PER_SECOND;
        }

        // For Timestamp nanos should be in the range [0, 999999999].
        if self.nanos < 0 {
            self.seconds -= 1;
            self.nanos += NANOS_PER_SECOND;
        }

        // TODO: should this be checked?
        // debug_assert!(self.seconds >= -62_135_596_800 && self.seconds <= 253_402_300_799,
        //               "invalid timestamp: {:?}", self);
    }

    pub fn new(seconds: i64, nanos: i32) -> Self {
        let mut ts = Timestamp {
            seconds,
            nanos
        };
        ts.normalize();
        ts
    }

}

/// Converts a `std::time::SystemTime` to a `Timestamp`.
impl From<time::SystemTime> for Timestamp {
    fn from(time: time::SystemTime) -> Timestamp {
        let duration = Duration::from(time.duration_since(time::UNIX_EPOCH).unwrap());
        Timestamp {
            seconds: duration.seconds,
            nanos: duration.nanos,
        }
    }
}

impl TryFrom<Timestamp> for time::SystemTime {
    type Error = time::Duration;

    /// Converts a `Timestamp` to a `SystemTime`, or if the timestamp falls before the Unix epoch,
    /// a duration containing the difference.
    fn try_from(mut timestamp: Timestamp) -> Result<time::SystemTime, time::Duration> {
        timestamp.normalize();
        if timestamp.seconds >= 0 {
            Ok(time::UNIX_EPOCH
                + time::Duration::new(timestamp.seconds as u64, timestamp.nanos as u32))
        } else {
            let mut duration = Duration {
                seconds: -timestamp.seconds,
                nanos: timestamp.nanos,
            };
            duration.normalize();
            Err(time::Duration::new(
                duration.seconds as u64,
                duration.nanos as u32,
            ))
        }
    }
}

impl Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> where
        S: Serializer {
        let mut ts = Timestamp {
            seconds: self.seconds,
            nanos: self.nanos
        };
        ts.normalize();
        let dt = chrono::NaiveDateTime::from_timestamp(self.seconds, self.nanos as u32);
        let utc: DateTime<Utc> = chrono::DateTime::from_utc(dt, chrono::Utc);
        serializer.serialize_str(format!("{:?}", utc).as_str())
    }
}

impl<'de> Deserialize<'de> for Timestamp  {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error> where
        D: Deserializer<'de> {

        struct TimestampVisitor;

        impl<'de> Visitor<'de> for TimestampVisitor {
            type Value = Timestamp;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Timestamp in RFC3339 format")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                where
                    E: de::Error,
            {
                let utc: DateTime<Utc> = chrono::DateTime::from_str(value).unwrap();
                let ts = Timestamp {
                    seconds: utc.timestamp(),
                    nanos: utc.timestamp_subsec_nanos() as i32
                };
                Ok(ts)
            }
        }
        deserializer.deserialize_str(TimestampVisitor)
    }
}

/// Value Convenience Methods
///
/// A collection of methods to make working with value easier.
///

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueError {
    description: Cow<'static, str>,
}

impl ValueError {
    pub fn new<S>(description: S) -> Self
    where
        S: Into<Cow<'static, str>>,
    {
        ValueError {
            description: description.into(),
        }
    }
}

impl std::error::Error for ValueError {
    fn description(&self) -> &str {
        &self.description
    }
}

impl std::fmt::Display for ValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("failed to convert Value: ")?;
        f.write_str(&self.description)
    }
}

impl Value {
    pub fn null() -> Self {
        let kind = Some(value::Kind::NullValue(0));
        Value { kind }
    }
    pub fn number(num: f64) -> Self {
        Value::from(num)
    }
    pub fn string(s: String) -> Self {
        Value::from(s)
    }
    pub fn bool(b: bool) -> Self {
        Value::from(b)
    }
    pub fn pb_struct(m: std::collections::BTreeMap<std::string::String, Value>) -> Self {
        Value::from(m)
    }
    pub fn pb_list(l: std::vec::Vec<Value>) -> Self {
        Value::from(l)
    }
}

impl From<NullValue> for Value {
    fn from(_: NullValue) -> Self {
        Value::null()
    }
}

impl From<f64> for Value {
    fn from(num: f64) -> Self {
        let kind = Some(value::Kind::NumberValue(num));
        Value { kind }
    }
}

impl TryFrom<Value> for f64 {
    type Error = ValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value.kind {
            Some(value::Kind::NumberValue(num)) => Ok(num),
            Some(other) => Err(ValueError::new(format!(
                "Cannot convert to f64 because this is not a ValueNumber. We got instead a {:?}",
                other
            ))),
            _ => Err(ValueError::new(
                "Conversion to f64 failed because value is empty!",
            )),
        }
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        let kind = Some(value::Kind::StringValue(s));
        Value { kind }
    }
}

impl TryFrom<Value> for String {
    type Error = ValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value.kind {
            Some(value::Kind::StringValue(string)) => Ok(string),
            Some(other) => Err(ValueError::new(format!(
                "Cannot convert to String because this is not a StringValue. We got instead a {:?}",
                other
            ))),
            _ => Err(ValueError::new(
                "Conversion to String failed because value is empty!",
            )),
        }
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        let kind = Some(value::Kind::BoolValue(b));
        Value { kind }
    }
}

impl TryFrom<Value> for bool {
    type Error = ValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value.kind {
            Some(value::Kind::BoolValue(b)) => Ok(b),
            Some(other) => Err(ValueError::new(format!(
                "Cannot convert to bool because this is not a BoolValue. We got instead a {:?}",
                other
            ))),
            _ => Err(ValueError::new(
                "Conversion to bool failed because value is empty!",
            )),
        }
    }
}

impl From<std::collections::BTreeMap<std::string::String, Value>> for Value {
    fn from(fields: std::collections::BTreeMap<String, Value>) -> Self {
        let s = Struct { fields };
        let kind = Some(value::Kind::StructValue(s));
        Value { kind }
    }
}

impl TryFrom<Value> for std::collections::BTreeMap<std::string::String, Value> {
    type Error = ValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value.kind {
            Some(value::Kind::StructValue(s)) => Ok(s.fields),
            Some(other) => Err(ValueError::new(format!(
                "Cannot convert to BTreeMap<String, Value> because this is not a StructValue. We got instead a {:?}",
                other
            ))),
            _ => Err(ValueError::new(
                "Conversion to BTreeMap<String, Value> failed because value is empty!",
            )),
        }
    }
}

impl From<std::vec::Vec<Value>> for Value {
    fn from(values: Vec<Value>) -> Self {
        let v = ListValue { values };
        let kind = Some(value::Kind::ListValue(v));
        Value { kind }
    }
}

impl TryFrom<Value> for std::vec::Vec<Value> {
    type Error = ValueError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value.kind {
            Some(value::Kind::ListValue(list)) => Ok(list.values),
            Some(other) => Err(ValueError::new(format!(
                "Cannot convert to Vec<Value> because this is not a ListValue. We got instead a {:?}",
                other
            ))),
            _ => Err(ValueError::new(
                "Conversion to Vec<Value> failed because value is empty!",
            )),
        }
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> where
        S: Serializer {
        match &self.kind {
            Some(value::Kind::NumberValue(num)) => serializer.serialize_f64(num.clone()),
            Some(value::Kind::StringValue(string)) => serializer.serialize_str(&string),
            Some(value::Kind::BoolValue(boolean)) => serializer.serialize_bool(boolean.clone()),
            Some(value::Kind::NullValue(_)) => serializer.serialize_none(),
            Some(value::Kind::ListValue(list)) => {
                let mut seq = serializer.serialize_seq(Some(list.values.len()))?;
                for e in list.clone().values {
                    seq.serialize_element(&e)?;
                }
                seq.end()
            },
            Some(value::Kind::StructValue(object)) => {
                let mut map = serializer.serialize_map(Some(object.fields.len()))?;
                for (k, v) in object.clone().fields {
                    map.serialize_entry(&k, &v)?;
                }
                map.end()
            },
            _ => serializer.serialize_none()
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error> where
        D: Deserializer<'de> {

        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = crate::Value;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a prost_types::Value struct")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
                where
                    E: de::Error,
            {
                Ok(Value::from(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
                where
                    E: de::Error,
            {
                Ok(Value::from(value as f64))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
                where
                    E: de::Error,
            {
                Ok(Value::from(value as f64))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
                where
                    E: de::Error,
            {
                Ok(Value::from(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                where
                    E: de::Error,
            {
                Ok(Value::from(String::from(value)))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
                where
                    E: de::Error,
            {
                Ok(Value::from(value))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
                where
                    E: de::Error,
            {
                Ok(Value::null())
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
                where
                    E: de::Error,
            {
                Ok(Value::null())
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where
                    A: SeqAccess<'de>,
            {
                let mut values: Vec<Value> = Vec::new();
                while let Some(el) = seq.next_element()? {
                    values.push(el)
                }
                Ok(Value::from(values))
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where
                    A: MapAccess<'de>,
            {
                let mut fields: std::collections::BTreeMap<String, Value> = BTreeMap::new();
                while let Some((key, value)) = map.next_entry()? {
                    fields.insert(key, value);
                }
                Ok(Value::from(fields))
            }

        }
        deserializer.deserialize_any(ValueVisitor)
    }
}

/// Any Convenience Methods
///
/// Pack and unpack for Any value
///

use prost::MessageSerde;
use serde_json::json;

impl Any {
    // A type_url can take the format of `type.googleapis.com/package_name.struct_name`
    pub fn pack<T>(message: T) -> Self
    where
        T: prost::Message + prost::MessageMeta + Default
    {
        let type_url= prost::MessageMeta::type_url(&message).to_string();
        // Serialize the message into a value
        let mut buf = Vec::new();
        buf.reserve(message.encoded_len());
        message.encode(&mut buf).unwrap();
        Any {
            type_url,
            value: buf,
        }
    }

    pub fn unpack<T: prost::Message>(self, mut target: T) -> Result<T, prost::DecodeError> {
        let mut cursor = std::io::Cursor::new(self.value.as_slice());
        target.merge(&mut cursor).map(|_| target)
    }
}

impl Serialize for Any {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error> where
        S: Serializer {
        let type_url = self.type_url.clone();
        let empty = json!({
            "@type": type_url,
            "value": {}
        });
        let template: Box<dyn MessageSerde> = serde_json::from_value(empty).unwrap();
        match template.new_instance(self.value.clone()) {
            Ok(result) => {
                serde::ser::Serialize::serialize(result.as_ref(), serializer)
            },
            Err(_) => {
                let mut state = serializer.serialize_struct("Any", 3)?;
                state.serialize_field("@type", &self.type_url)?;
                state.serialize_field("value", &self.value)?;
                state.end()
            }

        }

    }
}

impl<'de> Deserialize<'de> for Any  {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where
        D: Deserializer<'de>,
    {
        let erased: Box<dyn MessageSerde> = serde::de::Deserialize::deserialize(deserializer).unwrap();
        let type_url = erased.type_url().to_string();
        let value = erased.encoded();
        Ok(
            Any {
                type_url,
                value
            }
        )
    }
}
