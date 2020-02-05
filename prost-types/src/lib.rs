#![doc(html_root_url = "https://docs.rs/prost-types/0.6.1")]

//! Protocol Buffers well-known types.
//!
//! Note that the documentation for the types defined in this crate are generated from the Protobuf
//! definitions, so code examples are not in Rust.
//!
//! See the [Protobuf reference][1] for more information about well-known types.
//!
//! [1]: https://developers.google.com/protocol-buffers/docs/reference/google.protobuf

use std::convert::TryFrom;
use std::i32;
use std::i64;
use std::time;

use chrono::prelude::*;
use std::borrow::Cow;

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

    pub fn to_datetime(&self) -> DateTime<Utc> {
        let dt = NaiveDateTime::from_timestamp(self.seconds, self.nanos as u32);
        DateTime::from_utc(dt, Utc)
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

/// Converts chrono's `NaiveDateTime` to `Timestamp`..
impl From<NaiveDateTime> for Timestamp {
    fn from(dt: NaiveDateTime) -> Self {
        Timestamp {
            seconds: dt.timestamp(),
            nanos: dt.timestamp_subsec_nanos() as i32
        }
    }
}

/// Converts chrono's `DateTime<UTtc>` to `Timestamp`..
impl From<DateTime<Utc>> for Timestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        Timestamp {
            seconds: dt.timestamp(),
            nanos: dt.timestamp_subsec_nanos() as i32
        }
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


