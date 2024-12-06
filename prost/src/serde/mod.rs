use core::marker::PhantomData;

use serde::{de::DeserializeSeed, Serialize};

use private::{CustomDeserialize, CustomSerialize};

#[doc(hidden)]
pub mod private;

#[doc(hidden)]
pub mod ser;

#[doc(hidden)]
pub mod de;

#[doc(hidden)]
pub mod types;

pub trait SerdeMessage: CustomSerialize + for<'de> CustomDeserialize<'de> {}

impl<T> SerdeMessage for T where T: CustomSerialize + for<'de> CustomDeserialize<'de> {}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SerializerConfig {
    pub emit_fields_with_default_value: bool,
    pub emit_nulled_optional_fields: bool,
    pub emit_enum_values_as_integer: bool,
    pub use_proto_name: bool,
}

impl SerializerConfig {
    #[inline]
    pub fn with<'a, T>(&'a self, val: &'a T) -> WithSerializerConfig<'a, T> {
        WithSerializerConfig {
            inner: val,
            config: self,
        }
    }
}

#[derive(Debug)]
pub struct WithSerializerConfig<'a, T> {
    inner: &'a T,
    config: &'a SerializerConfig,
}

impl<'a, T> WithSerializerConfig<'a, T>
where
    T: private::CustomSerialize,
{
    #[inline]
    pub fn config(&self) -> &SerializerConfig {
        self.config
    }

    #[cfg(feature = "serde-json")]
    #[inline]
    pub fn to_string(self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }

    #[cfg(feature = "serde-json")]
    #[inline]
    pub fn to_string_pretty(self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self)
    }

    #[cfg(feature = "serde-json")]
    #[inline]
    pub fn to_vec(self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(&self)
    }

    #[cfg(feature = "serde-json")]
    #[inline]
    pub fn to_vec_pretty(self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec_pretty(&self)
    }

    #[cfg(feature = "serde-json")]
    #[inline]
    pub fn to_writer<W: std::io::Write>(self, writer: W) -> Result<(), serde_json::Error> {
        serde_json::to_writer_pretty(writer, &self)
    }

    #[cfg(feature = "serde-json")]
    #[inline]
    pub fn to_writer_pretty<W: std::io::Write>(self, writer: W) -> Result<(), serde_json::Error> {
        serde_json::to_writer_pretty(writer, &self)
    }
}

impl<'a, T> Serialize for WithSerializerConfig<'a, T>
where
    T: private::CustomSerialize,
{
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        private::CustomSerialize::serialize(self.inner, serializer, self.config)
    }
}

pub trait WithSerializerConfigExt: Sized {
    #[inline]
    fn with_config<'a>(&'a self, config: &'a SerializerConfig) -> WithSerializerConfig<'a, Self> {
        WithSerializerConfig {
            inner: self,
            config,
        }
    }
}

impl<T> WithSerializerConfigExt for T where T: private::CustomSerialize {}

#[derive(Debug, Clone)]
pub struct SerializerConfigBuilder {
    config: SerializerConfig,
}

impl SerializerConfigBuilder {
    #[inline]
    pub fn new() -> Self {
        Self {
            config: Default::default(),
        }
    }

    #[inline]
    pub fn emit_fields_with_default_value(mut self, emit: bool) -> Self {
        self.config.emit_fields_with_default_value = emit;
        self
    }

    #[inline]
    pub fn emit_nulled_optional_fields(mut self, emit: bool) -> Self {
        self.config.emit_nulled_optional_fields = emit;
        self
    }

    #[inline]
    pub fn emit_enum_values_as_integer(mut self, emit: bool) -> Self {
        self.config.emit_enum_values_as_integer = emit;
        self
    }

    #[inline]
    pub fn use_proto_name(mut self, enabled: bool) -> Self {
        self.config.use_proto_name = enabled;
        self
    }

    #[inline]
    pub fn build(self) -> SerializerConfig {
        self.config
    }
}

impl Default for SerializerConfigBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct DeserializerConfig {
    pub ignore_unknown_fields: bool,
    pub ignore_unknown_enum_string_values: bool,
    pub deny_unknown_enum_values: bool,
}

impl DeserializerConfig {
    #[cfg(feature = "serde-json")]
    pub fn deserialize_from_str<T>(&self, val: &str) -> Result<T, serde_json::Error>
    where
        T: for<'de> private::CustomDeserialize<'de>,
    {
        let mut deserializer = serde_json::Deserializer::from_str(val);
        let val = <T as private::CustomDeserialize>::deserialize(&mut deserializer, self)?;
        deserializer.end()?;
        Ok(val)
    }

    #[cfg(feature = "serde-json")]
    pub fn deserialize_from_slice<T>(&self, val: &[u8]) -> Result<T, serde_json::Error>
    where
        T: for<'de> private::CustomDeserialize<'de>,
    {
        let mut deserializer = serde_json::Deserializer::from_slice(val);
        let val = <T as private::CustomDeserialize>::deserialize(&mut deserializer, self)?;
        deserializer.end()?;
        Ok(val)
    }

    #[cfg(feature = "serde-json")]
    pub fn deserialize_from_reader<T, R>(&self, val: R) -> Result<T, serde_json::Error>
    where
        R: std::io::Read,
        T: for<'de> private::CustomDeserialize<'de>,
    {
        let mut deserializer = serde_json::Deserializer::from_reader(val);
        let val = <T as private::CustomDeserialize>::deserialize(&mut deserializer, self)?;
        deserializer.end()?;
        Ok(val)
    }

    #[cfg(feature = "serde-json")]
    pub fn deserialize_from_value<T>(
        &self,
        value: &serde_json::Value,
    ) -> Result<T, serde_json::Error>
    where
        T: for<'de> private::CustomDeserialize<'de>,
    {
        <T as private::CustomDeserialize>::deserialize(value, self)
    }

    #[inline]
    pub fn with<T>(self) -> WithDeserializerConfig<T>
    where
        T: for<'de> CustomDeserialize<'de>,
    {
        WithDeserializerConfig {
            config: self,
            _for: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct WithDeserializerConfig<T> {
    config: DeserializerConfig,
    _for: PhantomData<T>,
}

impl<'de, T> DeserializeSeed<'de> for WithDeserializerConfig<T>
where
    T: CustomDeserialize<'de>,
{
    type Value = T;

    #[inline]
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        <T as CustomDeserialize>::deserialize(deserializer, &self.config)
    }
}

#[derive(Debug, Clone)]
pub struct DeserializerConfigBuilder {
    config: DeserializerConfig,
}

impl DeserializerConfigBuilder {
    #[inline]
    pub fn new() -> Self {
        Self {
            config: Default::default(),
        }
    }

    #[inline]
    pub fn ignore_unknown_fields(mut self, ignore: bool) -> Self {
        self.config.ignore_unknown_fields = ignore;
        self
    }

    #[inline]
    pub fn deny_unknown_enum_values(mut self, deny: bool) -> Self {
        self.config.deny_unknown_enum_values = deny;
        self
    }

    #[inline]
    pub fn ignore_unknown_enum_string_values(mut self, ignore: bool) -> Self {
        self.config.ignore_unknown_enum_string_values = ignore;
        self
    }

    #[inline]
    pub fn build(self) -> DeserializerConfig {
        self.config
    }
}

impl Default for DeserializerConfigBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
