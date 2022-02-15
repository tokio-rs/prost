#![doc(html_root_url = "https://docs.rs/prost-types/0.9.0")]

//! Protocol Buffers well-known types.
//!
//! Note that the documentation for the types defined in this crate are generated from the Protobuf
//! definitions, so code examples are not in Rust.
//!
//! See the [Protobuf reference][1] for more information about well-known types.
//!
//! [1]: https://developers.google.com/protocol-buffers/docs/reference/google.protobuf

#![cfg_attr(not(feature = "std"), no_std)]

use core::convert::TryFrom;
use core::i32;
use core::i64;
use core::time;

include!("protobuf.rs");
pub mod compiler {
    include!("compiler.rs");
}

// The Protobuf `Duration` and `Timestamp` types can't delegate to the standard library equivalents
// because the Protobuf versions are signed. To make them easier to work with, `From` conversions
// are defined in both directions.

const NANOS_PER_SECOND: i32 = 1_000_000_000;
const NANOS_MAX: i32 = NANOS_PER_SECOND - 1;

impl Duration {
    /// Normalizes the duration to a canonical format.
    ///
    /// Based on [`google::protobuf::util::CreateNormalized`][1].
    /// [1]: https://github.com/google/protobuf/blob/v3.3.2/src/google/protobuf/util/time_util.cc#L79-L100
    pub fn normalize(&mut self) {
        // Make sure nanos is in the range.
        if self.nanos <= -NANOS_PER_SECOND || self.nanos >= NANOS_PER_SECOND {
            if let Some(seconds) = self
                .seconds
                .checked_add((self.nanos / NANOS_PER_SECOND) as i64)
            {
                self.seconds = seconds;
                self.nanos %= NANOS_PER_SECOND;
            } else if self.nanos < 0 {
                // Negative overflow! Set to the least normal value.
                self.seconds = i64::MIN;
                self.nanos = -NANOS_MAX;
            } else {
                // Positive overflow! Set to the greatest normal value.
                self.seconds = i64::MAX;
                self.nanos = NANOS_MAX;
            }
        }

        // nanos should have the same sign as seconds.
        if self.seconds < 0 && self.nanos > 0 {
            if let Some(seconds) = self.seconds.checked_add(1) {
                self.seconds = seconds;
                self.nanos -= NANOS_PER_SECOND;
            } else {
                // Positive overflow! Set to the greatest normal value.
                debug_assert_eq!(self.seconds, i64::MAX);
                self.nanos = NANOS_MAX;
            }
        } else if self.seconds > 0 && self.nanos < 0 {
            if let Some(seconds) = self.seconds.checked_sub(1) {
                self.seconds = seconds;
                self.nanos += NANOS_PER_SECOND;
            } else {
                // Negative overflow! Set to the least normal value.
                debug_assert_eq!(self.seconds, i64::MIN);
                self.nanos = -NANOS_MAX;
            }
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
    #[cfg(feature = "std")]
    pub fn normalize(&mut self) {
        // Make sure nanos is in the range.
        if self.nanos <= -NANOS_PER_SECOND || self.nanos >= NANOS_PER_SECOND {
            if let Some(seconds) = self
                .seconds
                .checked_add((self.nanos / NANOS_PER_SECOND) as i64)
            {
                self.seconds = seconds;
                self.nanos %= NANOS_PER_SECOND;
            } else if self.nanos < 0 {
                // Negative overflow! Set to the earliest normal value.
                self.seconds = i64::MIN;
                self.nanos = 0;
            } else {
                // Positive overflow! Set to the latest normal value.
                self.seconds = i64::MAX;
                self.nanos = 999_999_999;
            }
        }

        // For Timestamp nanos should be in the range [0, 999999999].
        if self.nanos < 0 {
            if let Some(seconds) = self.seconds.checked_sub(1) {
                self.seconds = seconds;
                self.nanos += NANOS_PER_SECOND;
            } else {
                // Negative overflow! Set to the earliest normal value.
                debug_assert_eq!(self.seconds, i64::MIN);
                self.nanos = 0;
            }
        }

        // TODO: should this be checked?
        // debug_assert!(self.seconds >= -62_135_596_800 && self.seconds <= 253_402_300_799,
        //               "invalid timestamp: {:?}", self);
    }
}

/// Implements the unstable/naive version of `Eq`: a basic equality check on the internal fields of the `Timestamp`.
/// This implies that `normalized_ts != non_normalized_ts` even if `normalized_ts == non_normalized_ts.normalized()`.
#[cfg(feature = "std")]
impl Eq for Timestamp {}

#[cfg(feature = "std")]
#[allow(clippy::derive_hash_xor_eq)] // Derived logic is correct: comparing the 2 feilds for equality
impl std::hash::Hash for Timestamp {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.seconds.hash(state);
        self.nanos.hash(state);
    }
}

#[cfg(feature = "std")]
impl From<std::time::SystemTime> for Timestamp {
    fn from(system_time: std::time::SystemTime) -> Timestamp {
        let (seconds, nanos) = match system_time.duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => {
                let seconds = i64::try_from(duration.as_secs()).unwrap();
                (seconds, duration.subsec_nanos() as i32)
            }
            Err(error) => {
                let duration = error.duration();
                let seconds = i64::try_from(duration.as_secs()).unwrap();
                let nanos = duration.subsec_nanos() as i32;
                if nanos == 0 {
                    (-seconds, 0)
                } else {
                    (-seconds - 1, 1_000_000_000 - nanos)
                }
            }
        };
        Timestamp { seconds, nanos }
    }
}

/// Indicates that a [`Timestamp`] could not be converted to
/// [`SystemTime`][std::time::SystemTime] because it is out of range.
///
/// The range of times that can be represented by `SystemTime` depends on the platform.
/// All `Timestamp`s are likely representable on 64-bit Unix-like platforms, but
/// other platforms, such as Windows and 32-bit Linux, may not be able to represent
/// the full range of `Timestamp`s.
#[cfg(feature = "std")]
#[derive(Debug)]
#[non_exhaustive]
pub struct TimestampOutOfSystemRangeError {
    pub timestamp: Timestamp,
}

#[cfg(feature = "std")]
impl core::fmt::Display for TimestampOutOfSystemRangeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:?} is not representable as a `SystemTime` because it is out of range",
            self
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TimestampOutOfSystemRangeError {}

#[cfg(feature = "std")]
impl TryFrom<Timestamp> for std::time::SystemTime {
    type Error = TimestampOutOfSystemRangeError;

    fn try_from(mut timestamp: Timestamp) -> Result<std::time::SystemTime, Self::Error> {
        let orig_timestamp = timestamp.clone();
        timestamp.normalize();

        let system_time = if timestamp.seconds >= 0 {
            std::time::UNIX_EPOCH.checked_add(time::Duration::from_secs(timestamp.seconds as u64))
        } else {
            std::time::UNIX_EPOCH
                .checked_sub(time::Duration::from_secs((-timestamp.seconds) as u64))
        };

        let system_time = system_time.and_then(|system_time| {
            system_time.checked_add(time::Duration::from_nanos(timestamp.nanos as u64))
        });

        system_time.ok_or(TimestampOutOfSystemRangeError {
            timestamp: orig_timestamp,
        })
    }
}

#[cfg(feature = "std")]
impl serde::Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(
            &humantime::format_rfc3339(std::time::SystemTime::try_from(self.clone()).unwrap())
                .to_string(),
        )
    }
}

struct TimestampVisitor;

#[cfg(feature = "std")]
impl<'de> serde::de::Visitor<'de> for TimestampVisitor {
    type Value = Timestamp;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a valid RFC 3339 timestamp string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Timestamp::from(
            humantime::parse_rfc3339(value).map_err(serde::de::Error::custom)?,
        ))
    }
}

impl<'de> serde::Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Timestamp, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(TimestampVisitor)
    }
}

pub trait HasConstructor {
    fn new() -> Self;
}

pub struct MyType<'de, T: serde::de::Visitor<'de> + HasConstructor>(
    <T as serde::de::Visitor<'de>>::Value,
);

impl<'de, T> serde::Deserialize<'de> for MyType<'de, T>
where
    T: serde::de::Visitor<'de> + HasConstructor,
{
    fn deserialize<D>(deserializer: D) -> Result<MyType<'de, T>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer
            .deserialize_any(T::new())
            .map(|x| MyType { 0: x })
    }
}

pub fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

pub mod vec_visitor {
    struct VecVisitor<'de, T>
    where
        T: serde::Deserialize<'de>,
    {
        _vec_type: &'de std::marker::PhantomData<T>,
    }

    #[cfg(feature = "std")]
    impl<'de, T: serde::Deserialize<'de>> serde::de::Visitor<'de> for VecVisitor<'de, T> {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid String string or integer")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut res = Self::Value::with_capacity(seq.size_hint().unwrap_or(0));
            loop {
                match seq.next_element()? {
                    Some(el) => res.push(el),
                    None => return Ok(res),
                }
            }
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Self::Value::default())
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D, T: 'de + serde::Deserialize<'de>>(
        deserializer: D,
    ) -> Result<Vec<T>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(VecVisitor::<'de, T> {
            _vec_type: &std::marker::PhantomData,
        })
    }
}

pub mod repeated_visitor {
    struct VecVisitor<'de, T>
    where
        T: serde::de::Visitor<'de> + crate::HasConstructor,
    {
        _vec_type: &'de std::marker::PhantomData<T>,
    }

    #[cfg(feature = "std")]
    impl<'de, T> serde::de::Visitor<'de> for VecVisitor<'de, T>
    where
        T: serde::de::Visitor<'de> + crate::HasConstructor,
    {
        type Value = Vec<<T as serde::de::Visitor<'de>>::Value>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid String string or integer")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut res = Self::Value::with_capacity(seq.size_hint().unwrap_or(0));
            loop {
                let response: std::option::Option<crate::MyType<'de, T>> = seq.next_element()?;
                match response {
                    Some(el) => res.push(el.0),
                    None => return Ok(res),
                }
            }
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Self::Value::default())
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D, T: 'de + serde::de::Visitor<'de> + crate::HasConstructor>(
        deserializer: D,
    ) -> Result<Vec<<T as serde::de::Visitor<'de>>::Value>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(VecVisitor::<'de, T> {
            _vec_type: &std::marker::PhantomData,
        })
    }

    pub fn serialize<S, F>(
        value: &Vec<<F as crate::SerializeMethod>::Value>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        F: crate::SerializeMethod,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(value.len()))?;
        for e in value {
            seq.serialize_element(&crate::MySeType::<F> { val: e })?;
        }
        seq.end()
    }
}

pub mod enum_visitor {
    struct EnumVisitor<'de, T>
    where
        T: ToString
            + std::str::FromStr
            + std::convert::Into<i32>
            + std::convert::TryFrom<i32>
            + Default,
    {
        _type: &'de std::marker::PhantomData<T>,
    }

    #[cfg(feature = "std")]
    impl<'de, T> serde::de::Visitor<'de> for EnumVisitor<'de, T>
    where
        T: ToString
            + std::str::FromStr
            + std::convert::Into<i32>
            + std::convert::TryFrom<i32>
            + Default,
    {
        type Value = i32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid String string or integer")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match T::from_str(value) {
                Ok(en) => Ok(en.into()),
                Err(_) => Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(value),
                    &self,
                )),
            }
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Self::Value::default())
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<i32, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: 'de
            + ToString
            + std::str::FromStr
            + std::convert::Into<i32>
            + std::convert::TryFrom<i32>
            + Default,
    {
        deserializer.deserialize_any(EnumVisitor::<'de, T> {
            _type: &std::marker::PhantomData,
        })
    }

    pub fn serialize<S, T>(value: &i32, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        T: ToString
            + std::str::FromStr
            + std::convert::Into<i32>
            + std::convert::TryFrom<i32>
            + Default,
    {
        match T::try_from(*value) {
            Err(_) => Err(serde::ser::Error::custom("invalid enum value")),
            Ok(t) => serializer.serialize_str(&t.to_string()),
        }
    }
}

pub mod map_custom_serializer {
    pub fn serialize<S, K, G>(
        value: &std::collections::HashMap<K, <G as crate::SerializeMethod>::Value>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        K: serde::Serialize + std::cmp::Eq + std::hash::Hash,
        G: crate::SerializeMethod,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(value.len()))?;
        for (key, value) in value {
            map.serialize_entry(&key, &crate::MySeType::<G> { val: value })?;
        }
        map.end()
    }
}

pub mod btree_map_custom_serializer {
    pub fn serialize<S, K, G>(
        value: &std::collections::BTreeMap<K, <G as crate::SerializeMethod>::Value>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        K: serde::Serialize + std::cmp::Eq + std::cmp::Ord,
        G: crate::SerializeMethod,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(value.len()))?;
        for (key, value) in value {
            map.serialize_entry(&key, &crate::MySeType::<G> { val: value })?;
        }
        map.end()
    }
}

pub mod map_custom_visitor {
    struct MapVisitor<'de, T, V>
    where
        T: serde::de::Visitor<'de> + crate::HasConstructor,
        V: serde::Deserialize<'de>,
    {
        _map_type: fn() -> (
            std::marker::PhantomData<&'de T>,
            std::marker::PhantomData<&'de V>,
        ),
    }

    #[cfg(feature = "std")]
    impl<'de, T, V> serde::de::Visitor<'de> for MapVisitor<'de, T, V>
    where
        T: serde::de::Visitor<'de> + crate::HasConstructor,
        V: serde::Deserialize<'de>,
        <T as serde::de::Visitor<'de>>::Value: std::cmp::Eq + std::hash::Hash,
    {
        type Value = std::collections::HashMap<<T as serde::de::Visitor<'de>>::Value, V>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid String string or integer")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut res = Self::Value::with_capacity(map.size_hint().unwrap_or(0));
            loop {
                let response: std::option::Option<(crate::MyType<'de, T>, V)> = map.next_entry()?;
                match response {
                    Some((key, val)) => {
                        res.insert(key.0, val);
                    }
                    _ => return Ok(res),
                }
            }
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Self::Value::default())
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D, T, V>(
        deserializer: D,
    ) -> Result<std::collections::HashMap<<T as serde::de::Visitor<'de>>::Value, V>, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: 'de + serde::de::Visitor<'de> + crate::HasConstructor,
        V: 'de + serde::Deserialize<'de>,
        <T as serde::de::Visitor<'de>>::Value: std::cmp::Eq + std::hash::Hash,
    {
        deserializer.deserialize_any(MapVisitor::<'de, T, V> {
            _map_type: || (std::marker::PhantomData, std::marker::PhantomData),
        })
    }

    pub fn serialize<S, F, V>(
        value: &std::collections::HashMap<<F as crate::SerializeMethod>::Value, V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        F: crate::SerializeMethod,
        V: serde::Serialize,
        <F as crate::SerializeMethod>::Value: std::cmp::Eq + std::hash::Hash,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(value.len()))?;
        for (key, value) in value {
            map.serialize_entry(&crate::MySeType::<F> { val: key }, &value)?;
        }
        map.end()
    }
}

pub mod map_custom_to_custom_visitor {
    struct MapVisitor<'de, T, S>
    where
        T: serde::de::Visitor<'de> + crate::HasConstructor,
        S: serde::de::Visitor<'de> + crate::HasConstructor,
    {
        _map_type: fn() -> (
            std::marker::PhantomData<&'de T>,
            std::marker::PhantomData<&'de S>,
        ),
    }

    #[cfg(feature = "std")]
    impl<'de, T, S> serde::de::Visitor<'de> for MapVisitor<'de, T, S>
    where
        T: serde::de::Visitor<'de> + crate::HasConstructor,
        S: serde::de::Visitor<'de> + crate::HasConstructor,
        <T as serde::de::Visitor<'de>>::Value: std::cmp::Eq + std::hash::Hash,
    {
        type Value = std::collections::HashMap<
            <T as serde::de::Visitor<'de>>::Value,
            <S as serde::de::Visitor<'de>>::Value,
        >;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid String string or integer")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut res = Self::Value::with_capacity(map.size_hint().unwrap_or(0));
            loop {
                let response: std::option::Option<(crate::MyType<'de, T>, crate::MyType<'de, S>)> =
                    map.next_entry()?;
                match response {
                    Some((key, val)) => {
                        res.insert(key.0, val.0);
                    }
                    _ => return Ok(res),
                }
            }
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Self::Value::default())
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D, T, S>(
        deserializer: D,
    ) -> Result<
        std::collections::HashMap<
            <T as serde::de::Visitor<'de>>::Value,
            <S as serde::de::Visitor<'de>>::Value,
        >,
        D::Error,
    >
    where
        D: serde::Deserializer<'de>,
        T: 'de + serde::de::Visitor<'de> + crate::HasConstructor,
        S: 'de + serde::de::Visitor<'de> + crate::HasConstructor,
        <T as serde::de::Visitor<'de>>::Value: std::cmp::Eq + std::hash::Hash,
    {
        deserializer.deserialize_any(MapVisitor::<'de, T, S> {
            _map_type: || (std::marker::PhantomData, std::marker::PhantomData),
        })
    }

    pub fn serialize<S, F, G>(
        value: &std::collections::HashMap<
            <F as crate::SerializeMethod>::Value,
            <G as crate::SerializeMethod>::Value,
        >,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        F: crate::SerializeMethod,
        G: crate::SerializeMethod,
        <F as crate::SerializeMethod>::Value: std::cmp::Eq + std::hash::Hash,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(value.len()))?;
        for (key, value) in value {
            map.serialize_entry(
                &crate::MySeType::<F> { val: key },
                &crate::MySeType::<G> { val: value },
            )?;
        }
        map.end()
    }
}

pub mod btree_map_custom_visitor {
    struct MapVisitor<'de, T, V>
    where
        T: serde::de::Visitor<'de> + crate::HasConstructor,
        V: serde::Deserialize<'de>,
    {
        _map_type: fn() -> (
            std::marker::PhantomData<&'de T>,
            std::marker::PhantomData<&'de V>,
        ),
    }

    #[cfg(feature = "std")]
    impl<'de, T, V> serde::de::Visitor<'de> for MapVisitor<'de, T, V>
    where
        T: serde::de::Visitor<'de> + crate::HasConstructor,
        V: serde::Deserialize<'de>,
        <T as serde::de::Visitor<'de>>::Value: std::cmp::Eq + std::cmp::Ord,
    {
        type Value = std::collections::BTreeMap<<T as serde::de::Visitor<'de>>::Value, V>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid String string or integer")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut res = Self::Value::new();
            loop {
                let response: std::option::Option<(crate::MyType<'de, T>, V)> = map.next_entry()?;
                match response {
                    Some((key, val)) => {
                        res.insert(key.0, val);
                    }
                    _ => return Ok(res),
                }
            }
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Self::Value::default())
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D, T, V>(
        deserializer: D,
    ) -> Result<std::collections::BTreeMap<<T as serde::de::Visitor<'de>>::Value, V>, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: 'de + serde::de::Visitor<'de> + crate::HasConstructor,
        V: 'de + serde::Deserialize<'de>,
        <T as serde::de::Visitor<'de>>::Value: std::cmp::Eq + std::cmp::Ord,
    {
        deserializer.deserialize_any(MapVisitor::<'de, T, V> {
            _map_type: || (std::marker::PhantomData, std::marker::PhantomData),
        })
    }

    pub fn serialize<S, F, V>(
        value: &std::collections::BTreeMap<<F as crate::SerializeMethod>::Value, V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        F: crate::SerializeMethod,
        V: serde::Serialize,
        <F as crate::SerializeMethod>::Value: std::cmp::Eq + std::cmp::Ord,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(value.len()))?;
        for (key, value) in value {
            map.serialize_entry(&crate::MySeType::<F> { val: key }, &value)?;
        }
        map.end()
    }
}

pub mod btree_map_custom_to_custom_visitor {
    struct MapVisitor<'de, T, S>
    where
        T: serde::de::Visitor<'de> + crate::HasConstructor,
        S: serde::de::Visitor<'de> + crate::HasConstructor,
    {
        _map_type: fn() -> (
            std::marker::PhantomData<&'de T>,
            std::marker::PhantomData<&'de S>,
        ),
    }

    #[cfg(feature = "std")]
    impl<'de, T, S> serde::de::Visitor<'de> for MapVisitor<'de, T, S>
    where
        T: serde::de::Visitor<'de> + crate::HasConstructor,
        S: serde::de::Visitor<'de> + crate::HasConstructor,
        <T as serde::de::Visitor<'de>>::Value: std::cmp::Eq + std::cmp::Ord,
    {
        type Value = std::collections::BTreeMap<
            <T as serde::de::Visitor<'de>>::Value,
            <S as serde::de::Visitor<'de>>::Value,
        >;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid String string or integer")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut res = Self::Value::new();
            loop {
                let response: std::option::Option<(crate::MyType<'de, T>, crate::MyType<'de, S>)> =
                    map.next_entry()?;
                match response {
                    Some((key, val)) => {
                        res.insert(key.0, val.0);
                    }
                    _ => return Ok(res),
                }
            }
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Self::Value::default())
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D, T, S>(
        deserializer: D,
    ) -> Result<
        std::collections::BTreeMap<
            <T as serde::de::Visitor<'de>>::Value,
            <S as serde::de::Visitor<'de>>::Value,
        >,
        D::Error,
    >
    where
        D: serde::Deserializer<'de>,
        T: 'de + serde::de::Visitor<'de> + crate::HasConstructor,
        S: 'de + serde::de::Visitor<'de> + crate::HasConstructor,
        <T as serde::de::Visitor<'de>>::Value: std::cmp::Eq + std::cmp::Ord,
    {
        deserializer.deserialize_any(MapVisitor::<'de, T, S> {
            _map_type: || (std::marker::PhantomData, std::marker::PhantomData),
        })
    }

    pub fn serialize<S, F, G>(
        value: &std::collections::BTreeMap<
            <F as crate::SerializeMethod>::Value,
            <G as crate::SerializeMethod>::Value,
        >,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        F: crate::SerializeMethod,
        G: crate::SerializeMethod,
        <F as crate::SerializeMethod>::Value: std::cmp::Eq + std::cmp::Ord,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(value.len()))?;
        for (key, value) in value {
            map.serialize_entry(
                &crate::MySeType::<F> { val: key },
                &crate::MySeType::<G> { val: value },
            )?;
        }
        map.end()
    }
}

pub trait SerializeMethod {
    type Value;
    fn serialize<S>(value: &Self::Value, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer;
}

pub struct MySeType<'a, T>
where
    T: SerializeMethod,
{
    val: &'a <T as SerializeMethod>::Value,
}

impl<'a, T: SerializeMethod> serde::Serialize for MySeType<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        T::serialize(self.val, serializer)
    }
}

pub mod map_visitor {
    struct MapVisitor<'de, K, V>
    where
        K: serde::Deserialize<'de> + std::cmp::Eq + std::hash::Hash,
        V: serde::Deserialize<'de>,
    {
        _key_type: &'de std::marker::PhantomData<K>,
        _value_type: &'de std::marker::PhantomData<V>,
    }

    #[cfg(feature = "std")]
    impl<
            'de,
            K: serde::Deserialize<'de> + std::cmp::Eq + std::hash::Hash,
            V: serde::Deserialize<'de>,
        > serde::de::Visitor<'de> for MapVisitor<'de, K, V>
    {
        type Value = std::collections::HashMap<K, V>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid String string or integer")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut res = Self::Value::with_capacity(map.size_hint().unwrap_or(0));
            loop {
                match map.next_entry()? {
                    Some((k, v)) => {
                        res.insert(k, v);
                    }
                    None => return Ok(res),
                }
            }
        }
        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Self::Value::default())
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<
        'de,
        D,
        K: 'de + serde::Deserialize<'de> + std::cmp::Eq + std::hash::Hash,
        V: 'de + serde::Deserialize<'de>,
    >(
        deserializer: D,
    ) -> Result<std::collections::HashMap<K, V>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(MapVisitor::<'de, K, V> {
            _key_type: &std::marker::PhantomData,
            _value_type: &std::marker::PhantomData,
        })
    }
}

pub mod btree_map_visitor {
    struct MapVisitor<'de, K, V>
    where
        K: serde::Deserialize<'de> + std::cmp::Eq + std::cmp::Ord,
        V: serde::Deserialize<'de>,
    {
        _key_type: &'de std::marker::PhantomData<K>,
        _value_type: &'de std::marker::PhantomData<V>,
    }

    #[cfg(feature = "std")]
    impl<
            'de,
            K: serde::Deserialize<'de> + std::cmp::Eq + std::cmp::Ord,
            V: serde::Deserialize<'de>,
        > serde::de::Visitor<'de> for MapVisitor<'de, K, V>
    {
        type Value = std::collections::BTreeMap<K, V>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid String string or integer")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut res = Self::Value::new();
            loop {
                match map.next_entry()? {
                    Some((k, v)) => {
                        res.insert(k, v);
                    }
                    None => return Ok(res),
                }
            }
        }
        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Self::Value::default())
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<
        'de,
        D,
        K: 'de + serde::Deserialize<'de> + std::cmp::Eq + std::cmp::Ord,
        V: 'de + serde::Deserialize<'de>,
    >(
        deserializer: D,
    ) -> Result<std::collections::BTreeMap<K, V>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(MapVisitor::<'de, K, V> {
            _key_type: &std::marker::PhantomData,
            _value_type: &std::marker::PhantomData,
        })
    }
}

pub mod string_visitor {
    struct StringVisitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for StringVisitor {
        type Value = std::string::String;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            return Ok(value.to_string());
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Self::Value::default())
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<std::string::String, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(StringVisitor)
    }
}

pub mod string_opt_visitor {
    struct StringVisitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for StringVisitor {
        type Value = std::option::Option<std::string::String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid String string or integer")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            return Ok(Some(value.to_string()));
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<std::option::Option<std::string::String>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(StringVisitor)
    }
}

pub mod bool_visitor {
    pub struct BoolVisitor;

    impl crate::HasConstructor for BoolVisitor {
        fn new() -> Self {
            return Self {};
        }
    }

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for BoolVisitor {
        type Value = bool;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid Bool string or integer")
        }

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            return Ok(value);
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(bool::default())
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(BoolVisitor)
    }
}

pub mod bool_opt_visitor {
    struct BoolVisitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for BoolVisitor {
        type Value = std::option::Option<bool>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid Bool string or integer")
        }

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            return Ok(Some(value));
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<std::option::Option<bool>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(BoolVisitor)
    }
}

pub mod i32_visitor {
    pub struct I32Visitor;

    impl crate::HasConstructor for I32Visitor {
        fn new() -> I32Visitor {
            return I32Visitor {};
        }
    }

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for I32Visitor {
        type Value = i32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid I32 string or integer")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            use std::convert::TryFrom;
            i32::try_from(value).map_err(E::custom)
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if (value.trunc() - value).abs() > f64::EPSILON
                || value > i32::MAX as f64
                || value < i32::MIN as f64
            {
                Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Float(value),
                    &self,
                ))
            } else {
                // This is a round number in the proper range, we can cast just fine.
                Ok(value as i32)
            }
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            use std::convert::TryFrom;
            i32::try_from(value).map_err(E::custom)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // If we have scientific notation or a decimal, parse float first.
            if value.contains('e') || value.contains('E') || value.ends_with(".0") {
                value
                    .parse::<f64>()
                    .map_err(E::custom)
                    .and_then(|x| self.visit_f64(x))
            } else {
                value.parse::<i32>().map_err(E::custom)
            }
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(i32::default())
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<i32, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(I32Visitor)
    }
}

pub mod i32_opt_visitor {
    struct I32Visitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for I32Visitor {
        type Value = std::option::Option<i32>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid I32 string or integer")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            use std::convert::TryFrom;
            i32::try_from(value).map(|x| Some(x)).map_err(E::custom)
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if (value.trunc() - value).abs() > f64::EPSILON
                || value > i32::MAX as f64
                || value < i32::MIN as f64
            {
                Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Float(value),
                    &self,
                ))
            } else {
                // This is a round number in the proper range, we can cast just fine.
                Ok(Some(value as i32))
            }
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            use std::convert::TryFrom;
            i32::try_from(value).map(|x| Some(x)).map_err(E::custom)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // If we have scientific notation or a decimal, parse float first.
            if value.contains('e') || value.contains('E') || value.ends_with(".0") {
                value
                    .parse::<f64>()
                    .map_err(E::custom)
                    .and_then(|x| self.visit_f64(x))
            } else {
                value.parse::<i32>().map(|x| Some(x)).map_err(E::custom)
            }
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<std::option::Option<i32>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(I32Visitor)
    }
}

pub mod i64_visitor {
    pub struct I64Visitor;

    impl crate::HasConstructor for I64Visitor {
        fn new() -> Self {
            return Self {};
        }
    }

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for I64Visitor {
        type Value = i64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid I64 string or integer")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value as i64)
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if (value.trunc() - value).abs() > f64::EPSILON
                || value > i64::MAX as f64
                || value < i64::MIN as f64
            {
                Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Float(value),
                    &self,
                ))
            } else {
                // This is a round number in the proper range, we can cast just fine.
                Ok(value as i64)
            }
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            use std::convert::TryFrom;
            i64::try_from(value).map_err(E::custom)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // If we have scientific notation or a decimal, parse float first.
            if value.contains('e') || value.contains('E') || value.ends_with(".0") {
                value
                    .parse::<f64>()
                    .map_err(E::custom)
                    .and_then(|x| self.visit_f64(x))
            } else {
                value.parse::<i64>().map_err(E::custom)
            }
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(i64::default())
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<i64, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(I64Visitor)
    }
}

pub mod i64_opt_visitor {
    struct I64Visitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for I64Visitor {
        type Value = std::option::Option<i64>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid I64 string or integer")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(value as i64))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if (value.trunc() - value).abs() > f64::EPSILON
                || value > i64::MAX as f64
                || value < i64::MIN as f64
            {
                Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Float(value),
                    &self,
                ))
            } else {
                // This is a round number in the proper range, we can cast just fine.
                Ok(Some(value as i64))
            }
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            use std::convert::TryFrom;
            i64::try_from(value).map(|x| Some(x)).map_err(E::custom)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // If we have scientific notation or a decimal, parse float first.
            if value.contains('e') || value.contains('E') || value.ends_with(".0") {
                value
                    .parse::<f64>()
                    .map_err(E::custom)
                    .and_then(|x| self.visit_f64(x))
            } else {
                value.parse::<i64>().map(|x| Some(x)).map_err(E::custom)
            }
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<std::option::Option<i64>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(I64Visitor)
    }
}

pub mod u32_visitor {
    pub struct U32Visitor;

    impl crate::HasConstructor for U32Visitor {
        fn new() -> Self {
            return Self {};
        }
    }

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for U32Visitor {
        type Value = u32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid U32 string or integer")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            use std::convert::TryFrom;
            u32::try_from(value).map_err(E::custom)
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if (value.trunc() - value).abs() > f64::EPSILON
                || value < 0.0
                || value > u32::MAX as f64
            {
                Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Float(value),
                    &self,
                ))
            } else {
                // This is a round number in the proper range, we can cast just fine.
                Ok(value as u32)
            }
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            use std::convert::TryFrom;
            u32::try_from(value).map_err(E::custom)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // If we have scientific notation or a decimal, parse float first.
            if value.contains('e') || value.contains('E') || value.ends_with(".0") {
                value
                    .parse::<f64>()
                    .map_err(E::custom)
                    .and_then(|x| self.visit_f64(x))
            } else {
                value.parse::<u32>().map_err(E::custom)
            }
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(u32::default())
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u32, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(U32Visitor)
    }
}

pub mod u32_opt_visitor {
    struct U32Visitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for U32Visitor {
        type Value = std::option::Option<u32>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid U32 string or integer")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            use std::convert::TryFrom;
            u32::try_from(value).map(|x| Some(x)).map_err(E::custom)
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if (value.trunc() - value).abs() > f64::EPSILON
                || value < 0.0
                || value > u32::MAX as f64
            {
                Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Float(value),
                    &self,
                ))
            } else {
                // This is a round number in the proper range, we can cast just fine.
                Ok(Some(value as u32))
            }
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            use std::convert::TryFrom;
            u32::try_from(value).map(|x| Some(x)).map_err(E::custom)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // If we have scientific notation or a decimal, parse float first.
            if value.contains('e') || value.contains('E') || value.ends_with(".0") {
                value
                    .parse::<f64>()
                    .map_err(E::custom)
                    .and_then(|x| self.visit_f64(x))
            } else {
                value.parse::<u32>().map(|x| Some(x)).map_err(E::custom)
            }
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<std::option::Option<u32>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(U32Visitor)
    }
}

pub mod u64_visitor {
    pub struct U64Visitor;

    impl crate::HasConstructor for U64Visitor {
        fn new() -> Self {
            return Self {};
        }
    }

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for U64Visitor {
        type Value = u64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid U64 string or integer")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value as u64)
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if (value.trunc() - value).abs() > f64::EPSILON
                || value < 0.0
                || value > u64::MAX as f64
            {
                Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Float(value),
                    &self,
                ))
            } else {
                // This is a round number in the proper range, we can cast just fine.
                Ok(value as u64)
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // If we have scientific notation or a decimal, parse float first.
            if value.contains('e') || value.contains('E') || value.ends_with(".0") {
                value
                    .parse::<f64>()
                    .map_err(E::custom)
                    .and_then(|x| self.visit_f64(x))
            } else {
                value.parse::<u64>().map_err(E::custom)
            }
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(u64::default())
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(U64Visitor)
    }
}

pub mod u64_opt_visitor {
    struct U64Visitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for U64Visitor {
        type Value = std::option::Option<u64>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid U64 string or integer")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(value as u64))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if (value.trunc() - value).abs() > f64::EPSILON
                || value < 0.0
                || value > u64::MAX as f64
            {
                Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Float(value),
                    &self,
                ))
            } else {
                // This is a round number, we can cast just fine.
                Ok(Some(value as u64))
            }
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // If we have scientific notation or a decimal, parse float first.
            if value.contains('e') || value.contains('E') || value.ends_with(".0") {
                value
                    .parse::<f64>()
                    .map_err(E::custom)
                    .and_then(|x| self.visit_f64(x))
            } else {
                value.parse::<u64>().map(|x| Some(x)).map_err(E::custom)
            }
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<std::option::Option<u64>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(U64Visitor)
    }
}

pub mod f64_visitor {
    pub struct F64Visitor;

    impl crate::HasConstructor for F64Visitor {
        fn new() -> F64Visitor {
            return F64Visitor {};
        }
    }

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for F64Visitor {
        type Value = f64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid F64 string or integer")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value as f64)
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value as f64)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value as f64)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match value {
                "NaN" => Ok(f64::NAN),
                "Infinity" => Ok(f64::INFINITY),
                "-Infinity" => Ok(f64::NEG_INFINITY),
                _ => value.parse::<f64>().map_err(E::custom),
            }
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(f64::default())
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<f64, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(F64Visitor)
    }

    pub struct F64Serializer;

    impl crate::SerializeMethod for F64Serializer {
        type Value = f64;
        #[cfg(feature = "std")]
        fn serialize<S>(value: &Self::Value, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            if value.is_nan() {
                serializer.serialize_str("NaN")
            } else if value.is_infinite() && value.is_sign_negative() {
                serializer.serialize_str("-Infinity")
            } else if value.is_infinite() {
                serializer.serialize_str("Infinity")
            } else {
                serializer.serialize_f64(*value)
            }
        }
    }
}

pub mod f64_opt_visitor {
    struct F64Visitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for F64Visitor {
        type Value = std::option::Option<f64>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid F64 string or integer")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(value as f64))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(value as f64))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(value as f64))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match value {
                "NaN" => Ok(Some(f64::NAN)),
                "Infinity" => Ok(Some(f64::INFINITY)),
                "-Infinity" => Ok(Some(f64::NEG_INFINITY)),
                _ => value.parse::<f64>().map(|x| Some(x)).map_err(E::custom),
            }
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<std::option::Option<f64>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(F64Visitor)
    }

    #[cfg(feature = "std")]
    pub fn serialize<S>(value: &std::option::Option<f64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use crate::SerializeMethod;
        match value {
            None => serializer.serialize_none(),
            Some(double) => crate::f64_visitor::F64Serializer::serialize(double, serializer),
        }
    }
}

pub mod f32_visitor {
    pub struct F32Visitor;

    impl crate::HasConstructor for F32Visitor {
        fn new() -> F32Visitor {
            return F32Visitor {};
        }
    }

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for F32Visitor {
        type Value = f32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid F32 string or integer")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value as f32)
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if value < f32::MIN as f64 || value > f32::MAX as f64 {
                Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Float(value),
                    &self,
                ))
            } else {
                Ok(value as f32)
            }
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(value as f32)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match value {
                "NaN" => Ok(f32::NAN),
                "Infinity" => Ok(f32::INFINITY),
                "-Infinity" => Ok(f32::NEG_INFINITY),
                _ => value.parse::<f32>().map_err(E::custom),
            }
        }
        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(f32::default())
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<f32, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(F32Visitor)
    }

    pub struct F32Serializer;

    impl crate::SerializeMethod for F32Serializer {
        type Value = f32;

        #[cfg(feature = "std")]
        fn serialize<S>(value: &f32, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            if value.is_nan() {
                serializer.serialize_str("NaN")
            } else if value.is_infinite() && value.is_sign_negative() {
                serializer.serialize_str("-Infinity")
            } else if value.is_infinite() {
                serializer.serialize_str("Infinity")
            } else {
                serializer.serialize_f32(*value)
            }
        }
    }
}

pub mod f32_opt_visitor {
    struct F32Visitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for F32Visitor {
        type Value = std::option::Option<f32>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid F32 string or integer")
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(value as f32))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if value < f32::MIN as f64 || value > f32::MAX as f64 {
                Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Float(value),
                    &self,
                ))
            } else {
                Ok(Some(value as f32))
            }
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(value as f32))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match value {
                "NaN" => Ok(Some(f32::NAN)),
                "Infinity" => Ok(Some(f32::INFINITY)),
                "-Infinity" => Ok(Some(f32::NEG_INFINITY)),
                _ => value.parse::<f32>().map(|x| Some(x)).map_err(E::custom),
            }
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<std::option::Option<f32>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(F32Visitor)
    }

    #[cfg(feature = "std")]
    pub fn serialize<S>(value: &std::option::Option<f32>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use crate::SerializeMethod;
        match value {
            None => serializer.serialize_none(),
            Some(float) => crate::f32_visitor::F32Serializer::serialize(float, serializer),
        }
    }
}

pub mod vec_u8_visitor {
    pub struct VecU8Visitor;

    impl crate::HasConstructor for VecU8Visitor {
        fn new() -> Self {
            return Self {};
        }
    }

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for VecU8Visitor {
        type Value = ::prost::alloc::vec::Vec<u8>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid Base64 encoded string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            base64::decode(value).map_err(E::custom)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Self::Value::default())
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<::prost::alloc::vec::Vec<u8>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(VecU8Visitor)
    }

    pub struct VecU8Serializer;

    impl crate::SerializeMethod for VecU8Serializer {
        type Value = ::prost::alloc::vec::Vec<u8>;

        #[cfg(feature = "std")]
        fn serialize<S>(value: &Self::Value, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_str(&base64::encode(value))
        }
    }
}

pub mod vec_u8_opt_visitor {
    struct VecU8Visitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for VecU8Visitor {
        type Value = std::option::Option<::prost::alloc::vec::Vec<u8>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid Base64 encoded string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            base64::decode(value)
                .map(|str| Some(str))
                .map_err(E::custom)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<std::option::Option<::prost::alloc::vec::Vec<u8>>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(VecU8Visitor)
    }

    #[cfg(feature = "std")]
    pub fn serialize<S>(
        value: &std::option::Option<::prost::alloc::vec::Vec<u8>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use crate::SerializeMethod;
        match value {
            None => serializer.serialize_none(),
            Some(value) => crate::vec_u8_visitor::VecU8Serializer::serialize(value, serializer),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use proptest::prelude::*;

    use super::*;

    #[cfg(feature = "std")]
    proptest! {
        #[test]
        fn check_system_time_roundtrip(
            system_time in SystemTime::arbitrary(),
        ) {
            prop_assert_eq!(SystemTime::try_from(Timestamp::from(system_time)).unwrap(), system_time);
        }

        #[test]
        fn check_timestamp_roundtrip_via_system_time(
            seconds in i64::arbitrary(),
            nanos in i32::arbitrary(),
        ) {
            let mut timestamp = Timestamp { seconds, nanos };
            timestamp.normalize();
            if let Ok(system_time) = SystemTime::try_from(timestamp.clone()) {
                prop_assert_eq!(Timestamp::from(system_time), timestamp);
            }
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn check_timestamp_negative_seconds() {
        // Representative tests for the case of timestamps before the UTC Epoch time:
        // validate the expected behaviour that "negative second values with fractions
        // must still have non-negative nanos values that count forward in time"
        // https://developers.google.com/protocol-buffers/docs/reference/google.protobuf#google.protobuf.Timestamp
        //
        // To ensure cross-platform compatibility, all nanosecond values in these
        // tests are in minimum 100 ns increments.  This does not affect the general
        // character of the behaviour being tested, but ensures that the tests are
        // valid for both POSIX (1 ns precision) and Windows (100 ns precision).
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - Duration::new(1_001, 0)),
            Timestamp {
                seconds: -1_001,
                nanos: 0
            }
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - Duration::new(0, 999_999_900)),
            Timestamp {
                seconds: -1,
                nanos: 100
            }
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - Duration::new(2_001_234, 12_300)),
            Timestamp {
                seconds: -2_001_235,
                nanos: 999_987_700
            }
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - Duration::new(768, 65_432_100)),
            Timestamp {
                seconds: -769,
                nanos: 934_567_900
            }
        );
    }

    #[cfg(all(unix, feature = "std"))]
    #[test]
    fn check_timestamp_negative_seconds_1ns() {
        // UNIX-only test cases with 1 ns precision
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - Duration::new(0, 999_999_999)),
            Timestamp {
                seconds: -1,
                nanos: 1
            }
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - Duration::new(1_234_567, 123)),
            Timestamp {
                seconds: -1_234_568,
                nanos: 999_999_877
            }
        );
        assert_eq!(
            Timestamp::from(UNIX_EPOCH - Duration::new(890, 987_654_321)),
            Timestamp {
                seconds: -891,
                nanos: 12_345_679
            }
        );
    }

    #[test]
    fn check_duration_normalize() {
        #[rustfmt::skip] // Don't mangle the table formatting.
        let cases = [
            // --- Table of test cases ---
            //        test seconds      test nanos  expected seconds  expected nanos
            (line!(),            0,              0,                0,              0),
            (line!(),            1,              1,                1,              1),
            (line!(),           -1,             -1,               -1,             -1),
            (line!(),            0,    999_999_999,                0,    999_999_999),
            (line!(),            0,   -999_999_999,                0,   -999_999_999),
            (line!(),            0,  1_000_000_000,                1,              0),
            (line!(),            0, -1_000_000_000,               -1,              0),
            (line!(),            0,  1_000_000_001,                1,              1),
            (line!(),            0, -1_000_000_001,               -1,             -1),
            (line!(),           -1,              1,                0,   -999_999_999),
            (line!(),            1,             -1,                0,    999_999_999),
            (line!(),           -1,  1_000_000_000,                0,              0),
            (line!(),            1, -1_000_000_000,                0,              0),
            (line!(), i64::MIN    ,              0,     i64::MIN    ,              0),
            (line!(), i64::MIN + 1,              0,     i64::MIN + 1,              0),
            (line!(), i64::MIN    ,              1,     i64::MIN + 1,   -999_999_999),
            (line!(), i64::MIN    ,  1_000_000_000,     i64::MIN + 1,              0),
            (line!(), i64::MIN    , -1_000_000_000,     i64::MIN    ,   -999_999_999),
            (line!(), i64::MIN + 1, -1_000_000_000,     i64::MIN    ,              0),
            (line!(), i64::MIN + 2, -1_000_000_000,     i64::MIN + 1,              0),
            (line!(), i64::MIN    , -1_999_999_998,     i64::MIN    ,   -999_999_999),
            (line!(), i64::MIN + 1, -1_999_999_998,     i64::MIN    ,   -999_999_998),
            (line!(), i64::MIN + 2, -1_999_999_998,     i64::MIN + 1,   -999_999_998),
            (line!(), i64::MIN    , -1_999_999_999,     i64::MIN    ,   -999_999_999),
            (line!(), i64::MIN + 1, -1_999_999_999,     i64::MIN    ,   -999_999_999),
            (line!(), i64::MIN + 2, -1_999_999_999,     i64::MIN + 1,   -999_999_999),
            (line!(), i64::MIN    , -2_000_000_000,     i64::MIN    ,   -999_999_999),
            (line!(), i64::MIN + 1, -2_000_000_000,     i64::MIN    ,   -999_999_999),
            (line!(), i64::MIN + 2, -2_000_000_000,     i64::MIN    ,              0),
            (line!(), i64::MIN    ,   -999_999_998,     i64::MIN    ,   -999_999_998),
            (line!(), i64::MIN + 1,   -999_999_998,     i64::MIN + 1,   -999_999_998),
            (line!(), i64::MAX    ,              0,     i64::MAX    ,              0),
            (line!(), i64::MAX - 1,              0,     i64::MAX - 1,              0),
            (line!(), i64::MAX    ,             -1,     i64::MAX - 1,    999_999_999),
            (line!(), i64::MAX    ,  1_000_000_000,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 1,  1_000_000_000,     i64::MAX    ,              0),
            (line!(), i64::MAX - 2,  1_000_000_000,     i64::MAX - 1,              0),
            (line!(), i64::MAX    ,  1_999_999_998,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 1,  1_999_999_998,     i64::MAX    ,    999_999_998),
            (line!(), i64::MAX - 2,  1_999_999_998,     i64::MAX - 1,    999_999_998),
            (line!(), i64::MAX    ,  1_999_999_999,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 1,  1_999_999_999,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 2,  1_999_999_999,     i64::MAX - 1,    999_999_999),
            (line!(), i64::MAX    ,  2_000_000_000,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 1,  2_000_000_000,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 2,  2_000_000_000,     i64::MAX    ,              0),
            (line!(), i64::MAX    ,    999_999_998,     i64::MAX    ,    999_999_998),
            (line!(), i64::MAX - 1,    999_999_998,     i64::MAX - 1,    999_999_998),
        ];

        for case in cases.iter() {
            let mut test_duration = crate::Duration {
                seconds: case.1,
                nanos: case.2,
            };
            test_duration.normalize();

            assert_eq!(
                test_duration,
                crate::Duration {
                    seconds: case.3,
                    nanos: case.4,
                },
                "test case on line {} doesn't match",
                case.0,
            );
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn check_timestamp_normalize() {
        // Make sure that `Timestamp::normalize` behaves correctly on and near overflow.
        #[rustfmt::skip] // Don't mangle the table formatting.
        let cases = [
            // --- Table of test cases ---
            //        test seconds      test nanos  expected seconds  expected nanos
            (line!(),            0,              0,                0,              0),
            (line!(),            1,              1,                1,              1),
            (line!(),           -1,             -1,               -2,    999_999_999),
            (line!(),            0,    999_999_999,                0,    999_999_999),
            (line!(),            0,   -999_999_999,               -1,              1),
            (line!(),            0,  1_000_000_000,                1,              0),
            (line!(),            0, -1_000_000_000,               -1,              0),
            (line!(),            0,  1_000_000_001,                1,              1),
            (line!(),            0, -1_000_000_001,               -2,    999_999_999),
            (line!(),           -1,              1,               -1,              1),
            (line!(),            1,             -1,                0,    999_999_999),
            (line!(),           -1,  1_000_000_000,                0,              0),
            (line!(),            1, -1_000_000_000,                0,              0),
            (line!(), i64::MIN    ,              0,     i64::MIN    ,              0),
            (line!(), i64::MIN + 1,              0,     i64::MIN + 1,              0),
            (line!(), i64::MIN    ,              1,     i64::MIN    ,              1),
            (line!(), i64::MIN    ,  1_000_000_000,     i64::MIN + 1,              0),
            (line!(), i64::MIN    , -1_000_000_000,     i64::MIN    ,              0),
            (line!(), i64::MIN + 1, -1_000_000_000,     i64::MIN    ,              0),
            (line!(), i64::MIN + 2, -1_000_000_000,     i64::MIN + 1,              0),
            (line!(), i64::MIN    , -1_999_999_998,     i64::MIN    ,              0),
            (line!(), i64::MIN + 1, -1_999_999_998,     i64::MIN    ,              0),
            (line!(), i64::MIN + 2, -1_999_999_998,     i64::MIN    ,              2),
            (line!(), i64::MIN    , -1_999_999_999,     i64::MIN    ,              0),
            (line!(), i64::MIN + 1, -1_999_999_999,     i64::MIN    ,              0),
            (line!(), i64::MIN + 2, -1_999_999_999,     i64::MIN    ,              1),
            (line!(), i64::MIN    , -2_000_000_000,     i64::MIN    ,              0),
            (line!(), i64::MIN + 1, -2_000_000_000,     i64::MIN    ,              0),
            (line!(), i64::MIN + 2, -2_000_000_000,     i64::MIN    ,              0),
            (line!(), i64::MIN    ,   -999_999_998,     i64::MIN    ,              0),
            (line!(), i64::MIN + 1,   -999_999_998,     i64::MIN    ,              2),
            (line!(), i64::MAX    ,              0,     i64::MAX    ,              0),
            (line!(), i64::MAX - 1,              0,     i64::MAX - 1,              0),
            (line!(), i64::MAX    ,             -1,     i64::MAX - 1,    999_999_999),
            (line!(), i64::MAX    ,  1_000_000_000,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 1,  1_000_000_000,     i64::MAX    ,              0),
            (line!(), i64::MAX - 2,  1_000_000_000,     i64::MAX - 1,              0),
            (line!(), i64::MAX    ,  1_999_999_998,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 1,  1_999_999_998,     i64::MAX    ,    999_999_998),
            (line!(), i64::MAX - 2,  1_999_999_998,     i64::MAX - 1,    999_999_998),
            (line!(), i64::MAX    ,  1_999_999_999,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 1,  1_999_999_999,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 2,  1_999_999_999,     i64::MAX - 1,    999_999_999),
            (line!(), i64::MAX    ,  2_000_000_000,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 1,  2_000_000_000,     i64::MAX    ,    999_999_999),
            (line!(), i64::MAX - 2,  2_000_000_000,     i64::MAX    ,              0),
            (line!(), i64::MAX    ,    999_999_998,     i64::MAX    ,    999_999_998),
            (line!(), i64::MAX - 1,    999_999_998,     i64::MAX - 1,    999_999_998),
        ];

        for case in cases.iter() {
            let mut test_timestamp = crate::Timestamp {
                seconds: case.1,
                nanos: case.2,
            };
            test_timestamp.normalize();

            assert_eq!(
                test_timestamp,
                crate::Timestamp {
                    seconds: case.3,
                    nanos: case.4,
                },
                "test case on line {} doesn't match",
                case.0,
            );
        }
    }
}
