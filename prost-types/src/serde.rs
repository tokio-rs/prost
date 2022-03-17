#[cfg(feature = "std")]
impl ::serde::Serialize for crate::Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        use std::convert::TryInto;
        serializer.serialize_str(
            &chrono::DateTime::<chrono::Utc>::from_utc(
                chrono::NaiveDateTime::from_timestamp(self.seconds, self.nanos.try_into().unwrap()),
                chrono::Utc,
            )
            .to_rfc3339(),
        )
    }
}

struct TimestampVisitor;

#[cfg(feature = "std")]
impl<'de> ::serde::de::Visitor<'de> for TimestampVisitor {
    type Value = crate::Timestamp;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a valid RFC 3339 timestamp string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: ::serde::de::Error,
    {
        use std::convert::TryInto;
        let dt = chrono::DateTime::parse_from_rfc3339(value)
            .map_err(::serde::de::Error::custom)?
            .naive_utc();
        Ok(crate::Timestamp::from(
            std::time::UNIX_EPOCH
                + std::time::Duration::new(
                    dt.timestamp()
                        .try_into()
                        .map_err(::serde::de::Error::custom)?,
                    dt.timestamp_subsec_nanos(),
                ),
        ))
    }
}

impl<'de> ::serde::Deserialize<'de> for crate::Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<crate::Timestamp, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(TimestampVisitor)
    }
}

impl ::serde::Serialize for crate::Duration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        let mut nanos = self.nanos;
        if nanos < 0 {
            nanos = -nanos;
        }

        while nanos > 0 && nanos % 1_000 == 0 {
            nanos /= 1_000;
        }

        if nanos == 0 {
            serializer.serialize_str(&format!("{}s", self.seconds))
        } else {
            serializer.serialize_str(&format!("{}.{}s", self.seconds, nanos))
        }
    }
}

struct DurationVisitor;

#[cfg(feature = "std")]
impl<'de> ::serde::de::Visitor<'de> for DurationVisitor {
    type Value = crate::Duration;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a valid duration string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: ::serde::de::Error,
    {
        let value = match value.strip_suffix('s') {
            Some(value) => value,
            None => {
                return Err(::serde::de::Error::custom(format!(
                    "invalid duration: {}",
                    value
                )))
            }
        };
        let seconds = value.parse::<f64>().map_err(::serde::de::Error::custom)?;

        if seconds.is_sign_negative() {
            let crate::Duration { seconds, nanos } =
                std::time::Duration::from_secs_f64(-seconds).into();

            Ok(crate::Duration {
                seconds: -seconds,
                nanos: -nanos,
            })
        } else {
            Ok(std::time::Duration::from_secs_f64(seconds).into())
        }
    }
}

impl<'de> ::serde::Deserialize<'de> for crate::Duration {
    fn deserialize<D>(deserializer: D) -> Result<crate::Duration, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(DurationVisitor)
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

pub mod empty {
    struct EmptyVisitor;
    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for EmptyVisitor {
        type Value = ();

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid empty object")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let tmp: std::option::Option<((), ())> = map.next_entry()?;
            if tmp.is_some() {
                Err(::serde::de::Error::custom("this is a message, not empty"))
            } else {
                Ok(())
            }
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<(), D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        deserializer.deserialize_any(EmptyVisitor)
    }

    pub fn serialize<S>(_: &(), serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let map = serializer.serialize_map(Some(0))?;
        map.end()
    }
}

pub mod empty_opt {
    struct EmptyVisitor;
    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for EmptyVisitor {
        type Value = std::option::Option<()>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid empty object")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let tmp: std::option::Option<((), ())> = map.next_entry()?;
            if tmp.is_some() {
                Err(::serde::de::Error::custom("this is a message, not empty"))
            } else {
                Ok(Some(()))
            }
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Some(()))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }
    }

    #[cfg(feature = "std")]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<std::option::Option<()>, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        deserializer.deserialize_any(EmptyVisitor)
    }

    #[cfg(feature = "std")]
    pub fn serialize<S>(opt: &std::option::Option<()>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        if opt.is_some() {
            let map = serializer.serialize_map(Some(0))?;
            map.end()
        } else {
            serializer.serialize_none()
        }
    }
}

pub mod vec {
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
            formatter.write_str("a valid list")
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

pub mod repeated {
    struct VecVisitor<'de, T>
    where
        T: serde::de::Visitor<'de> + crate::serde::HasConstructor,
    {
        _vec_type: &'de std::marker::PhantomData<T>,
    }

    #[cfg(feature = "std")]
    impl<'de, T> serde::de::Visitor<'de> for VecVisitor<'de, T>
    where
        T: serde::de::Visitor<'de> + crate::serde::HasConstructor,
    {
        type Value = Vec<<T as serde::de::Visitor<'de>>::Value>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid repeated field")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut res = Self::Value::with_capacity(seq.size_hint().unwrap_or(0));
            loop {
                let response: std::option::Option<crate::serde::MyType<'de, T>> =
                    seq.next_element()?;
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
    pub fn deserialize<'de, D, T: 'de + serde::de::Visitor<'de> + crate::serde::HasConstructor>(
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
        value: &Vec<<F as crate::serde::SerializeMethod>::Value>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        F: crate::serde::SerializeMethod,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(value.len()))?;
        for e in value {
            seq.serialize_element(&crate::serde::MySeType::<F> { val: e })?;
        }
        seq.end()
    }
}

pub mod enum_serde {
    pub struct EnumVisitor<'de, T>
    where
        T: ToString
            + std::str::FromStr
            + std::convert::Into<i32>
            + std::convert::TryFrom<i32>
            + Default,
    {
        _type: &'de std::marker::PhantomData<T>,
    }

    impl<T> crate::serde::HasConstructor for EnumVisitor<'_, T>
    where
        T: ToString
            + std::str::FromStr
            + std::convert::Into<i32>
            + std::convert::TryFrom<i32>
            + Default,
    {
        fn new() -> Self {
            return Self {
                _type: &std::marker::PhantomData,
            };
        }
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
            formatter.write_str("a valid string or integer representation of an enum")
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

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match T::try_from(value as i32) {
                Ok(en) => Ok(en.into()),
                // There is a test in the conformance tests:
                // Required.Proto3.JsonInput.EnumFieldUnknownValue.Validator
                // That implies this should return the default value, so we
                // will. This also helps when parsing a oneof, since this means
                // we won't fail to deserialize when we have an out of bounds
                // enum value.
                Err(_) => Ok(T::default().into()),
            }
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_i64(value as i64)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_i64(value as i64)
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

    pub struct EnumSerializer<T>
    where
        T: std::convert::TryFrom<i32> + ToString,
    {
        _type: std::marker::PhantomData<T>,
    }

    impl<T> crate::serde::SerializeMethod for EnumSerializer<T>
    where
        T: std::convert::TryFrom<i32> + ToString,
    {
        type Value = i32;

        fn serialize<S>(value: &i32, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            match T::try_from(*value) {
                Err(_) => Err(serde::ser::Error::custom("invalid enum value")),
                Ok(t) => serializer.serialize_str(&t.to_string()),
            }
        }
    }
}

pub mod enum_opt {
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
        type Value = std::option::Option<i32>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid string or integer representation of an enum")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match T::from_str(value) {
                Ok(en) => Ok(Some(en.into())),
                Err(_) => Err(serde::de::Error::invalid_value(
                    serde::de::Unexpected::Str(value),
                    &self,
                )),
            }
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match T::try_from(value as i32) {
                Ok(en) => Ok(Some(en.into())),
                // There is a test in the conformance tests:
                // Required.Proto3.JsonInput.EnumFieldUnknownValue.Validator
                // That implies this should return the default value, so we
                // will. This also helps when parsing a oneof, since this means
                // we won't fail to deserialize when we have an out of bounds
                // enum value.
                Err(_) => Ok(Some(T::default().into())),
            }
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_i64(value as i64)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_i64(value as i64)
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
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<std::option::Option<i32>, D::Error>
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

    pub fn serialize<S, T>(
        value: &std::option::Option<i32>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        T: ToString
            + std::str::FromStr
            + std::convert::Into<i32>
            + std::convert::TryFrom<i32>
            + Default,
    {
        use crate::serde::SerializeMethod;
        match value {
            None => serializer.serialize_none(),
            Some(enum_int) => {
                crate::serde::enum_serde::EnumSerializer::<T>::serialize(enum_int, serializer)
            }
        }
    }
}

pub mod btree_map_custom_value {
    struct MapVisitor<'de, T, V>
    where
        T: serde::Deserialize<'de>,
        V: serde::de::Visitor<'de> + crate::serde::HasConstructor,
    {
        _map_type: fn() -> (
            std::marker::PhantomData<&'de T>,
            std::marker::PhantomData<&'de V>,
        ),
    }

    #[cfg(feature = "std")]
    impl<'de, T, V> serde::de::Visitor<'de> for MapVisitor<'de, T, V>
    where
        T: serde::Deserialize<'de> + std::cmp::Eq + std::cmp::Ord,
        V: serde::de::Visitor<'de> + crate::serde::HasConstructor,
    {
        type Value = std::collections::BTreeMap<T, <V as serde::de::Visitor<'de>>::Value>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid map")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut res = Self::Value::new();
            loop {
                let response: std::option::Option<(T, crate::serde::MyType<'de, V>)> =
                    map.next_entry()?;
                match response {
                    Some((key, val)) => {
                        res.insert(key, val.0);
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
    ) -> Result<std::collections::BTreeMap<T, <V as serde::de::Visitor<'de>>::Value>, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: 'de + serde::Deserialize<'de> + std::cmp::Eq + std::cmp::Ord,
        V: 'de + serde::de::Visitor<'de> + crate::serde::HasConstructor,
    {
        deserializer.deserialize_any(MapVisitor::<'de, T, V> {
            _map_type: || (std::marker::PhantomData, std::marker::PhantomData),
        })
    }

    pub fn serialize<S, T, F>(
        value: &std::collections::BTreeMap<T, <F as crate::serde::SerializeMethod>::Value>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        T: serde::Serialize + std::cmp::Eq + std::cmp::Ord,
        F: crate::serde::SerializeMethod,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(value.len()))?;
        for (key, value) in value {
            map.serialize_entry(&key, &crate::serde::MySeType::<F> { val: value })?;
        }
        map.end()
    }
}

pub mod map_custom_value {
    struct MapVisitor<'de, T, V>
    where
        T: serde::Deserialize<'de>,
        V: serde::de::Visitor<'de> + crate::serde::HasConstructor,
    {
        _map_type: fn() -> (
            std::marker::PhantomData<&'de T>,
            std::marker::PhantomData<&'de V>,
        ),
    }

    #[cfg(feature = "std")]
    impl<'de, T, V> serde::de::Visitor<'de> for MapVisitor<'de, T, V>
    where
        T: serde::Deserialize<'de> + std::cmp::Eq + std::hash::Hash,
        V: serde::de::Visitor<'de> + crate::serde::HasConstructor,
    {
        type Value = std::collections::HashMap<T, <V as serde::de::Visitor<'de>>::Value>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid map")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut res = Self::Value::with_capacity(map.size_hint().unwrap_or(0));
            loop {
                let response: std::option::Option<(T, crate::serde::MyType<'de, V>)> =
                    map.next_entry()?;
                match response {
                    Some((key, val)) => {
                        res.insert(key, val.0);
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
    ) -> Result<std::collections::HashMap<T, <V as serde::de::Visitor<'de>>::Value>, D::Error>
    where
        D: serde::Deserializer<'de>,
        T: 'de + serde::Deserialize<'de> + std::cmp::Eq + std::hash::Hash,
        V: 'de + serde::de::Visitor<'de> + crate::serde::HasConstructor,
    {
        deserializer.deserialize_any(MapVisitor::<'de, T, V> {
            _map_type: || (std::marker::PhantomData, std::marker::PhantomData),
        })
    }

    pub fn serialize<S, T, F>(
        value: &std::collections::HashMap<T, <F as crate::serde::SerializeMethod>::Value>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        T: serde::Serialize + std::cmp::Eq + std::hash::Hash,
        F: crate::serde::SerializeMethod,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(value.len()))?;
        for (key, value) in value {
            map.serialize_entry(&key, &crate::serde::MySeType::<F> { val: value })?;
        }
        map.end()
    }
}

pub mod map_custom {
    struct MapVisitor<'de, T, V>
    where
        T: serde::de::Visitor<'de> + crate::serde::HasConstructor,
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
        T: serde::de::Visitor<'de> + crate::serde::HasConstructor,
        V: serde::Deserialize<'de>,
        <T as serde::de::Visitor<'de>>::Value: std::cmp::Eq + std::hash::Hash,
    {
        type Value = std::collections::HashMap<<T as serde::de::Visitor<'de>>::Value, V>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid map")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut res = Self::Value::with_capacity(map.size_hint().unwrap_or(0));
            loop {
                let response: std::option::Option<(crate::serde::MyType<'de, T>, V)> =
                    map.next_entry()?;
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
        T: 'de + serde::de::Visitor<'de> + crate::serde::HasConstructor,
        V: 'de + serde::Deserialize<'de>,
        <T as serde::de::Visitor<'de>>::Value: std::cmp::Eq + std::hash::Hash,
    {
        deserializer.deserialize_any(MapVisitor::<'de, T, V> {
            _map_type: || (std::marker::PhantomData, std::marker::PhantomData),
        })
    }

    pub fn serialize<S, F, V>(
        value: &std::collections::HashMap<<F as crate::serde::SerializeMethod>::Value, V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        F: crate::serde::SerializeMethod,
        V: serde::Serialize,
        <F as crate::serde::SerializeMethod>::Value: std::cmp::Eq + std::hash::Hash,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(value.len()))?;
        for (key, value) in value {
            map.serialize_entry(&crate::serde::MySeType::<F> { val: key }, &value)?;
        }
        map.end()
    }
}

pub mod map_custom_to_custom {
    struct MapVisitor<'de, T, S>
    where
        T: serde::de::Visitor<'de> + crate::serde::HasConstructor,
        S: serde::de::Visitor<'de> + crate::serde::HasConstructor,
    {
        _map_type: fn() -> (
            std::marker::PhantomData<&'de T>,
            std::marker::PhantomData<&'de S>,
        ),
    }

    #[cfg(feature = "std")]
    impl<'de, T, S> serde::de::Visitor<'de> for MapVisitor<'de, T, S>
    where
        T: serde::de::Visitor<'de> + crate::serde::HasConstructor,
        S: serde::de::Visitor<'de> + crate::serde::HasConstructor,
        <T as serde::de::Visitor<'de>>::Value: std::cmp::Eq + std::hash::Hash,
    {
        type Value = std::collections::HashMap<
            <T as serde::de::Visitor<'de>>::Value,
            <S as serde::de::Visitor<'de>>::Value,
        >;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid map")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut res = Self::Value::with_capacity(map.size_hint().unwrap_or(0));
            loop {
                let response: std::option::Option<(
                    crate::serde::MyType<'de, T>,
                    crate::serde::MyType<'de, S>,
                )> = map.next_entry()?;
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
        T: 'de + serde::de::Visitor<'de> + crate::serde::HasConstructor,
        S: 'de + serde::de::Visitor<'de> + crate::serde::HasConstructor,
        <T as serde::de::Visitor<'de>>::Value: std::cmp::Eq + std::hash::Hash,
    {
        deserializer.deserialize_any(MapVisitor::<'de, T, S> {
            _map_type: || (std::marker::PhantomData, std::marker::PhantomData),
        })
    }

    pub fn serialize<S, F, G>(
        value: &std::collections::HashMap<
            <F as crate::serde::SerializeMethod>::Value,
            <G as crate::serde::SerializeMethod>::Value,
        >,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        F: crate::serde::SerializeMethod,
        G: crate::serde::SerializeMethod,
        <F as crate::serde::SerializeMethod>::Value: std::cmp::Eq + std::hash::Hash,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(value.len()))?;
        for (key, value) in value {
            map.serialize_entry(
                &crate::serde::MySeType::<F> { val: key },
                &crate::serde::MySeType::<G> { val: value },
            )?;
        }
        map.end()
    }
}

pub mod btree_map_custom {
    struct MapVisitor<'de, T, V>
    where
        T: serde::de::Visitor<'de> + crate::serde::HasConstructor,
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
        T: serde::de::Visitor<'de> + crate::serde::HasConstructor,
        V: serde::Deserialize<'de>,
        <T as serde::de::Visitor<'de>>::Value: std::cmp::Eq + std::cmp::Ord,
    {
        type Value = std::collections::BTreeMap<<T as serde::de::Visitor<'de>>::Value, V>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid map")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut res = Self::Value::new();
            loop {
                let response: std::option::Option<(crate::serde::MyType<'de, T>, V)> =
                    map.next_entry()?;
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
        T: 'de + serde::de::Visitor<'de> + crate::serde::HasConstructor,
        V: 'de + serde::Deserialize<'de>,
        <T as serde::de::Visitor<'de>>::Value: std::cmp::Eq + std::cmp::Ord,
    {
        deserializer.deserialize_any(MapVisitor::<'de, T, V> {
            _map_type: || (std::marker::PhantomData, std::marker::PhantomData),
        })
    }

    pub fn serialize<S, F, V>(
        value: &std::collections::BTreeMap<<F as crate::serde::SerializeMethod>::Value, V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        F: crate::serde::SerializeMethod,
        V: serde::Serialize,
        <F as crate::serde::SerializeMethod>::Value: std::cmp::Eq + std::cmp::Ord,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(value.len()))?;
        for (key, value) in value {
            map.serialize_entry(&crate::serde::MySeType::<F> { val: key }, &value)?;
        }
        map.end()
    }
}

pub mod btree_map_custom_to_custom {
    struct MapVisitor<'de, T, S>
    where
        T: serde::de::Visitor<'de> + crate::serde::HasConstructor,
        S: serde::de::Visitor<'de> + crate::serde::HasConstructor,
    {
        _map_type: fn() -> (
            std::marker::PhantomData<&'de T>,
            std::marker::PhantomData<&'de S>,
        ),
    }

    #[cfg(feature = "std")]
    impl<'de, T, S> serde::de::Visitor<'de> for MapVisitor<'de, T, S>
    where
        T: serde::de::Visitor<'de> + crate::serde::HasConstructor,
        S: serde::de::Visitor<'de> + crate::serde::HasConstructor,
        <T as serde::de::Visitor<'de>>::Value: std::cmp::Eq + std::cmp::Ord,
    {
        type Value = std::collections::BTreeMap<
            <T as serde::de::Visitor<'de>>::Value,
            <S as serde::de::Visitor<'de>>::Value,
        >;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid map")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut res = Self::Value::new();
            loop {
                let response: std::option::Option<(
                    crate::serde::MyType<'de, T>,
                    crate::serde::MyType<'de, S>,
                )> = map.next_entry()?;
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
        T: 'de + serde::de::Visitor<'de> + crate::serde::HasConstructor,
        S: 'de + serde::de::Visitor<'de> + crate::serde::HasConstructor,
        <T as serde::de::Visitor<'de>>::Value: std::cmp::Eq + std::cmp::Ord,
    {
        deserializer.deserialize_any(MapVisitor::<'de, T, S> {
            _map_type: || (std::marker::PhantomData, std::marker::PhantomData),
        })
    }

    pub fn serialize<S, F, G>(
        value: &std::collections::BTreeMap<
            <F as crate::serde::SerializeMethod>::Value,
            <G as crate::serde::SerializeMethod>::Value,
        >,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        F: crate::serde::SerializeMethod,
        G: crate::serde::SerializeMethod,
        <F as crate::serde::SerializeMethod>::Value: std::cmp::Eq + std::cmp::Ord,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(value.len()))?;
        for (key, value) in value {
            map.serialize_entry(
                &crate::serde::MySeType::<F> { val: key },
                &crate::serde::MySeType::<G> { val: value },
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

pub mod map {
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
            formatter.write_str("a valid map")
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

pub mod btree_map {
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
            formatter.write_str("a valid map")
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

pub mod string {
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

pub mod string_opt {
    struct StringVisitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for StringVisitor {
        type Value = std::option::Option<std::string::String>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid string")
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

pub mod bool {
    pub struct BoolVisitor;

    impl crate::serde::HasConstructor for BoolVisitor {
        fn new() -> Self {
            return Self {};
        }
    }

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for BoolVisitor {
        type Value = bool;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid boolean")
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

pub mod bool_map_key {
    pub struct BoolVisitor;

    impl crate::serde::HasConstructor for BoolVisitor {
        fn new() -> Self {
            return Self {};
        }
    }

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for BoolVisitor {
        type Value = bool;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid boolean")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match value {
                "true" => Ok(true),
                "false" => Ok(false),
                _ => Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Str(value),
                    &self,
                )),
            }
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(BoolVisitor)
    }

    pub struct BoolKeySerializer;

    impl crate::serde::SerializeMethod for BoolKeySerializer {
        type Value = bool;
        #[cfg(feature = "std")]
        fn serialize<S>(value: &Self::Value, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            if *value {
                serializer.serialize_str("true")
            } else {
                serializer.serialize_str("false")
            }
        }
    }
}

pub mod bool_opt {
    struct BoolVisitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for BoolVisitor {
        type Value = std::option::Option<bool>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid boolean")
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

pub mod i32 {
    pub struct I32Visitor;

    impl crate::serde::HasConstructor for I32Visitor {
        fn new() -> I32Visitor {
            return I32Visitor {};
        }
    }

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for I32Visitor {
        type Value = i32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid i32")
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

pub mod i32_opt {
    struct I32Visitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for I32Visitor {
        type Value = std::option::Option<i32>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid i32")
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

pub mod i64 {
    pub struct I64Visitor;

    impl crate::serde::HasConstructor for I64Visitor {
        fn new() -> Self {
            return Self {};
        }
    }

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for I64Visitor {
        type Value = i64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid i64")
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

    pub struct I64Serializer;

    impl crate::serde::SerializeMethod for I64Serializer {
        type Value = i64;
        #[cfg(feature = "std")]
        fn serialize<S>(value: &Self::Value, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_str(&value.to_string())
        }
    }
}

pub mod i64_opt {
    struct I64Visitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for I64Visitor {
        type Value = std::option::Option<i64>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid i64")
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

    #[cfg(feature = "std")]
    pub fn serialize<S>(value: &std::option::Option<i64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use crate::serde::SerializeMethod;
        match value {
            None => serializer.serialize_none(),
            Some(double) => crate::serde::i64::I64Serializer::serialize(double, serializer),
        }
    }
}

pub mod u32 {
    pub struct U32Visitor;

    impl crate::serde::HasConstructor for U32Visitor {
        fn new() -> Self {
            return Self {};
        }
    }

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for U32Visitor {
        type Value = u32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid u32")
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

pub mod u32_opt {
    struct U32Visitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for U32Visitor {
        type Value = std::option::Option<u32>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid u32")
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

pub mod u64 {
    pub struct U64Visitor;

    impl crate::serde::HasConstructor for U64Visitor {
        fn new() -> Self {
            return Self {};
        }
    }

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for U64Visitor {
        type Value = u64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid u64")
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

    pub struct U64Serializer;

    impl crate::serde::SerializeMethod for U64Serializer {
        type Value = u64;
        #[cfg(feature = "std")]
        fn serialize<S>(value: &Self::Value, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_str(&value.to_string())
        }
    }
}

pub mod u64_opt {
    struct U64Visitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for U64Visitor {
        type Value = std::option::Option<u64>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid u64")
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

    #[cfg(feature = "std")]
    pub fn serialize<S>(value: &std::option::Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use crate::serde::SerializeMethod;
        match value {
            None => serializer.serialize_none(),
            Some(double) => crate::serde::u64::U64Serializer::serialize(double, serializer),
        }
    }
}

pub mod f64 {
    pub struct F64Visitor;

    impl crate::serde::HasConstructor for F64Visitor {
        fn new() -> F64Visitor {
            return F64Visitor {};
        }
    }

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for F64Visitor {
        type Value = f64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid f64")
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
            Ok(value)
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

    impl crate::serde::SerializeMethod for F64Serializer {
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

pub mod f64_opt {
    struct F64Visitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for F64Visitor {
        type Value = std::option::Option<f64>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid f64")
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
            Ok(Some(value))
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
        use crate::serde::SerializeMethod;
        match value {
            None => serializer.serialize_none(),
            Some(double) => crate::serde::f64::F64Serializer::serialize(double, serializer),
        }
    }
}

pub mod f32 {
    pub struct F32Visitor;

    impl crate::serde::HasConstructor for F32Visitor {
        fn new() -> F32Visitor {
            return F32Visitor {};
        }
    }

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for F32Visitor {
        type Value = f32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid f32")
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

    impl crate::serde::SerializeMethod for F32Serializer {
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

pub mod f32_opt {
    struct F32Visitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for F32Visitor {
        type Value = std::option::Option<f32>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid f32")
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
        use crate::serde::SerializeMethod;
        match value {
            None => serializer.serialize_none(),
            Some(float) => crate::serde::f32::F32Serializer::serialize(float, serializer),
        }
    }
}

pub mod vec_u8 {
    pub struct VecU8Visitor;

    impl crate::serde::HasConstructor for VecU8Visitor {
        fn new() -> Self {
            return Self {};
        }
    }

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for VecU8Visitor {
        type Value = ::prost::alloc::vec::Vec<u8>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid base64 encoded string")
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

    impl crate::serde::SerializeMethod for VecU8Serializer {
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

pub mod vec_u8_opt {
    struct VecU8Visitor;

    #[cfg(feature = "std")]
    impl<'de> serde::de::Visitor<'de> for VecU8Visitor {
        type Value = std::option::Option<::prost::alloc::vec::Vec<u8>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a valid base64 encoded string")
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
        use crate::serde::SerializeMethod;
        match value {
            None => serializer.serialize_none(),
            Some(value) => crate::serde::vec_u8::VecU8Serializer::serialize(value, serializer),
        }
    }
}
