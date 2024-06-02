pub use core::convert::TryFrom;
pub use core::default::Default;
pub use core::fmt;
pub use core::marker::PhantomData;
pub use core::option::Option;
pub use core::result::Result;

pub use Option::{None, Some};
pub use Result::{Err, Ok};

pub use ::serde as _serde;

#[cfg(feature = "serde-json")]
pub use serde_json::Value as JsonValue;

pub use super::{DeserializerConfig, SerializerConfig};

#[inline]
pub fn is_default_value<T: Default + PartialEq>(val: &T) -> bool {
    *val == T::default()
}

// Serialization utilities.

pub use super::ser::{
    CustomSerialize, SerAsDisplay, SerBytesAsBase64, SerEnum, SerFloat32, SerFloat64, SerIdentity,
    SerMappedMapItems, SerMappedVecItems, SerSerde, SerWithConfig, SerializeOneOf,
};

// Deserialization utilities.

pub use super::de::{
    BoolDeserializer, BytesDeserializer, CustomDeserialize, DefaultDeserializer, DesIntoWithConfig,
    DesWithConfig, DeserializeEnum, DeserializeInto, DeserializeOneOf, EnumDeserializer,
    FloatDeserializer, ForwardDeserializer, IntDeserializer, MapDeserializer, MessageDeserializer,
    OneOfDeserializer, OptionDeserializer, VecDeserializer, WellKnownDeserializer,
};
