use core::{any::Any as CoreAny, cell::RefCell, fmt::Debug};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock, RwLock},
};

use ::serde as _serde;
pub use prost::serde::private::JsonValue;
use prost::{
    bytes::{Buf, BufMut},
    serde::SerdeMessage,
    Message, Name,
};

use crate::smallbox::{smallbox, SmallBox};

mod private {
    pub trait Sealed {}
}

pub trait AnyValue: CoreAny + Message + private::Sealed {
    fn as_any(&self) -> &(dyn CoreAny + Send + Sync);

    fn as_mut_any(&mut self) -> &mut (dyn CoreAny + Send + Sync);

    fn as_message(&self) -> &(dyn Message + Send + Sync);

    fn as_mut_message(&mut self) -> &mut (dyn Message + Send + Sync);

    fn clone_value(&self) -> Box<dyn AnyValue>;

    fn cmp_any(&self, other: &dyn AnyValue) -> bool;

    fn as_erased_serialize<'a>(
        &'a self,
        config: &'a prost::serde::SerializerConfig,
    ) -> SmallBox<dyn erased_serde::Serialize + 'a>;

    fn encode_to_buf(&self, buf: &mut dyn BufMut);
}

impl<T: 'static + Message + SerdeMessage + PartialEq + Clone> AnyValue for T {
    fn as_any(&self) -> &(dyn CoreAny + Send + Sync) {
        self as _
    }

    fn as_mut_any(&mut self) -> &mut (dyn CoreAny + Send + Sync) {
        self as _
    }

    fn as_message(&self) -> &(dyn Message + Send + Sync) {
        self as _
    }

    fn as_mut_message(&mut self) -> &mut (dyn Message + Send + Sync) {
        self as _
    }

    fn clone_value(&self) -> Box<dyn AnyValue> {
        Box::new(self.clone()) as _
    }

    fn cmp_any(&self, other: &dyn AnyValue) -> bool {
        other.as_any().downcast_ref::<T>() == Some(self)
    }

    fn encode_to_buf(&self, mut buf: &mut dyn BufMut) {
        self.encode_raw(&mut buf)
    }

    fn as_erased_serialize<'a>(
        &'a self,
        config: &'a prost::serde::SerializerConfig,
    ) -> SmallBox<dyn erased_serde::Serialize + 'a> {
        smallbox!(prost::serde::private::SerWithConfig(self, config))
    }
}

impl<T: 'static + Message> private::Sealed for T {}

#[derive(Debug)]
enum Inner {
    Protobuf(Vec<u8>),
    Json(JsonValue),
    Dyn(Box<dyn AnyValue>),
}

#[derive(Debug)]
pub struct ProstAny {
    type_url: String,
    inner: Inner,
    cached: RwLock<Option<Box<dyn AnyValue>>>,
}

const CACHED_INIT: RwLock<Option<Box<dyn AnyValue>>> = RwLock::new(None);

impl Clone for ProstAny {
    fn clone(&self) -> Self {
        Self {
            type_url: self.type_url.clone(),
            inner: match &self.inner {
                Inner::Protobuf(value) => Inner::Protobuf(value.clone()),
                Inner::Json(value) => Inner::Json(value.clone()),
                Inner::Dyn(value) => Inner::Dyn(value.clone_value()),
            },
            cached: CACHED_INIT,
        }
    }
}

impl PartialEq for ProstAny {
    fn eq(&self, other: &Self) -> bool {
        self.type_url == other.type_url
            && match (&self.inner, &other.inner) {
                (Inner::Protobuf(value_a), Inner::Protobuf(value_b)) => value_a == value_b,
                (Inner::Json(value_a), Inner::Json(value_b)) => value_a == value_b,
                (Inner::Dyn(value_a), Inner::Dyn(value_b)) => {
                    AnyValue::cmp_any(&**value_a, &**value_b)
                }
                _ => false,
            }
    }
}

impl Default for ProstAny {
    fn default() -> Self {
        Self {
            type_url: Default::default(),
            inner: Inner::Protobuf(Default::default()),
            cached: CACHED_INIT,
        }
    }
}

impl Name for ProstAny {
    const PACKAGE: &'static str = crate::PACKAGE;
    const NAME: &'static str = "Any";

    fn type_url() -> String {
        crate::type_url_for::<Self>()
    }
}

impl ProstAny {
    pub fn type_url(&self) -> &str {
        &self.type_url
    }

    pub fn set_type_url(&mut self, type_url: String) -> &mut Self {
        self.type_url = type_url;
        self
    }

    pub fn any_value(&self) -> &dyn AnyValue {
        self.opt_any_value()
            .expect("any value has not been resolved yet")
    }

    pub fn mut_any_value(&mut self) -> &mut dyn AnyValue {
        self.opt_mut_any_value()
            .expect("any value has not been resolved yet")
    }

    pub fn opt_any_value(&self) -> Option<&dyn AnyValue> {
        match &self.inner {
            Inner::Dyn(value) => Some(&**value),
            _ => None,
        }
    }

    pub fn opt_mut_any_value(&mut self) -> Option<&mut dyn AnyValue> {
        match &mut self.inner {
            Inner::Dyn(value) => Some(&mut **value),
            _ => None,
        }
    }

    pub fn into_any_value(self) -> Box<dyn AnyValue> {
        self.try_into_any_value()
            .expect("any value has not been resolved yet")
    }

    pub fn try_into_any_value(self) -> Result<Box<dyn AnyValue>, Self> {
        match self.inner {
            Inner::Dyn(value) => Ok(value),
            _ => Err(self),
        }
    }

    pub fn from_msg<T>(msg: T) -> Self
    where
        T: 'static + Message + SerdeMessage + Name + PartialEq + Clone,
    {
        Self {
            type_url: T::type_url(),
            inner: Inner::Dyn(Box::new(msg) as _),
            cached: CACHED_INIT,
        }
    }

    pub fn deserialize_any(
        &self,
        serde_config: Option<&prost::serde::DeserializerConfig>,
    ) -> Result<Box<dyn AnyValue>, prost::DecodeError> {
        if let Inner::Dyn(value) = &self.inner {
            return Ok(value.clone_value());
        }

        let type_descriptor = self.find_type_descriptor().ok_or_else(|| {
            prost::DecodeError::new(format!("unresolved type url: {}", self.type_url()))
        })?;

        let default_serde_config;
        let serde_config = match serde_config {
            Some(config) => config,
            None => {
                default_serde_config = Default::default();
                &default_serde_config
            }
        };

        match &self.inner {
            Inner::Protobuf(value) => (type_descriptor.deserialize_protobuf)(&self.type_url, value),
            Inner::Json(value) => {
                (type_descriptor.deserialize_json)(&self.type_url, value, serde_config)
            }
            Inner::Dyn(_) => unreachable!(),
        }
    }

    pub fn deserialize_any_in_place<'a, 'b>(
        &'a mut self,
        serde_config: Option<&'b prost::serde::DeserializerConfig>,
    ) -> Result<&'a mut dyn AnyValue, prost::DecodeError> {
        // This doesn't work due to
        // https://rust-lang.github.io/rfcs/2094-nll.html#problem-case-3-conditional-control-flow-across-functions.
        //
        // if let Inner::Dyn(value) = &mut self.inner {
        //     return Ok(&mut **value);
        // }
        //
        // So have to do this weird workaround instead:
        let has_inner_value = matches!(&self.inner, Inner::Dyn(_));
        if !has_inner_value {
            let value = self.deserialize_any(serde_config)?;

            self.inner = Inner::Dyn(value);
            self.cached = CACHED_INIT;
        }

        let Inner::Dyn(value) = &mut self.inner else {
            unreachable!()
        };

        Ok(&mut **value)
    }

    fn find_type_descriptor(&self) -> Option<AnyTypeDescriptor> {
        CURRENT_TYPE_RESOLVER.with(|type_resolver| {
            let type_resolver = type_resolver.borrow();
            let type_resolver = type_resolver.as_ref()?;
            Some(
                type_resolver
                    .resolve_message_type(self.type_url())
                    .ok()?
                    .clone(),
            )
        })
    }

    fn deserialize_and_cache<F, R>(&self, f: F) -> Result<R, prost::DecodeError>
    where
        F: FnOnce(&dyn AnyValue) -> R,
    {
        if let Inner::Dyn(value) = &self.inner {
            return Ok(f(&**value));
        }

        if let Some(value) = &*self.cached.read().unwrap() {
            return Ok(f(&**value));
        }

        let value = self.deserialize_any(None)?;
        let res = f(&*value);

        if let Ok(mut cached) = self.cached.try_write() {
            *cached = Some(value);
        }

        Ok(res)
    }
}

impl Message for ProstAny {
    fn encode_raw(&self, buf: &mut impl BufMut)
    where
        Self: Sized,
    {
        if !self.type_url.is_empty() {
            prost::encoding::string::encode(1u32, &self.type_url, buf);
        }

        match &self.inner {
            Inner::Protobuf(value) => {
                if !value.is_empty() {
                    prost::encoding::bytes::encode(2u32, value, buf);
                }
            }
            Inner::Dyn(value) => {
                prost::encoding::encode_key(2u32, prost::encoding::WireType::LengthDelimited, buf);
                prost::encoding::encode_varint(value.as_message().encoded_len() as u64, buf);
                value.encode_to_buf(buf);
            }
            Inner::Json(_) => {
                let res = self.deserialize_and_cache(|value| {
                    prost::encoding::encode_key(
                        2u32,
                        prost::encoding::WireType::LengthDelimited,
                        buf,
                    );
                    prost::encoding::encode_varint(value.as_message().encoded_len() as u64, buf);
                    value.encode_to_buf(buf);
                });
                if let Err(err) = res {
                    panic!("unresolved any value: {}", err)
                }
            }
        };
    }

    fn merge_field(
        &mut self,
        tag: u32,
        wire_type: prost::encoding::WireType,
        buf: &mut impl Buf,
        ctx: prost::encoding::DecodeContext,
    ) -> Result<(), prost::DecodeError>
    where
        Self: Sized,
    {
        match tag {
            1u32 => {
                let value = &mut self.type_url;
                ::prost::encoding::string::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push("Any", "type_url");
                    error
                })
            }
            2u32 => {
                let value = match &mut self.inner {
                    Inner::Protobuf(value) => value,
                    inner => {
                        *inner = Inner::Protobuf(Default::default());
                        let Inner::Protobuf(value) = inner else {
                            unreachable!()
                        };
                        value
                    }
                };
                ::prost::encoding::bytes::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push("Any", "value");
                    error
                })
            }
            _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;

        if !self.type_url.is_empty() {
            len += prost::encoding::string::encoded_len(1u32, &self.type_url);
        }

        match &self.inner {
            Inner::Protobuf(value) => {
                if !value.is_empty() {
                    len += prost::encoding::bytes::encoded_len(2u32, value);
                }
            }
            Inner::Dyn(value) => {
                len += prost::encoding::message::encoded_len(2u32, value.as_message());
            }
            Inner::Json(_) => {
                let res = self.deserialize_and_cache(|value| {
                    len += prost::encoding::message::encoded_len(2u32, value.as_message());
                });
                if let Err(err) = res {
                    panic!("unresolved any value: {}", err)
                }
            }
        }

        len
    }

    fn clear(&mut self) {
        self.type_url.clear();
        match &mut self.inner {
            Inner::Protobuf(value) => value.clear(),
            Inner::Dyn(value) => value.as_mut_message().clear(),
            Inner::Json(_) => {
                panic!("cannot clear unresolved type")
            }
        }
    }
}

impl prost::serde::private::CustomSerialize for ProstAny {
    fn serialize<S>(
        &self,
        serializer: S,
        config: &prost::serde::SerializerConfig,
    ) -> Result<S::Ok, S::Error>
    where
        S: _serde::Serializer,
    {
        let is_well_known_type = has_known_value_json_mapping(&self.type_url);

        #[derive(Debug, _serde::Serialize)]
        struct Flattened<'a, T: ?Sized> {
            #[serde(rename = "@type")]
            type_url: &'a str,
            #[serde(flatten)]
            value: &'a T,
        }

        #[derive(Debug, _serde::Serialize)]
        struct Wrapped<'a, T: ?Sized> {
            #[serde(rename = "@type")]
            type_url: &'a str,
            value: &'a T,
        }

        match &self.inner {
            Inner::Json(value) => {
                if is_well_known_type {
                    _serde::Serialize::serialize(
                        &Wrapped {
                            type_url: &self.type_url,
                            value,
                        },
                        serializer,
                    )
                } else {
                    _serde::Serialize::serialize(
                        &Flattened {
                            type_url: &self.type_url,
                            value,
                        },
                        serializer,
                    )
                }
            }
            Inner::Dyn(value) => {
                let value = &*value.as_erased_serialize(config);
                if is_well_known_type {
                    erased_serde::serialize(
                        &Wrapped {
                            type_url: &self.type_url,
                            value,
                        },
                        serializer,
                    )
                } else {
                    erased_serde::serialize(
                        &Flattened {
                            type_url: &self.type_url,
                            value,
                        },
                        serializer,
                    )
                }
            }
            Inner::Protobuf(_) => match self.deserialize_any(None) {
                Ok(value) => {
                    let value = &*value.as_erased_serialize(config);
                    if is_well_known_type {
                        erased_serde::serialize(
                            &Wrapped {
                                type_url: &self.type_url,
                                value,
                            },
                            serializer,
                        )
                    } else {
                        erased_serde::serialize(
                            &Flattened {
                                type_url: &self.type_url,
                                value,
                            },
                            serializer,
                        )
                    }
                }
                Err(err) => Err(_serde::ser::Error::custom(format!(
                    "failed to decode any value: {}",
                    err
                ))),
            },
        }
    }
}

impl<'de> prost::serde::private::CustomDeserialize<'de> for ProstAny {
    fn deserialize<D>(
        deserializer: D,
        config: &prost::serde::DeserializerConfig,
    ) -> Result<Self, D::Error>
    where
        D: _serde::Deserializer<'de>,
    {
        use _serde::de::{Error, Unexpected};

        let val = <JsonValue as _serde::Deserialize>::deserialize(deserializer)?;

        let JsonValue::Object(mut obj) = val else {
            return Err(D::Error::invalid_type(
                Unexpected::Other("non-object value"),
                &"object value",
            ));
        };
        let Some(JsonValue::String(type_url)) = obj.remove("@type") else {
            return Err(D::Error::missing_field("@type"));
        };

        let obj = if has_known_value_json_mapping(&type_url) {
            let Some(value) = obj.remove("value") else {
                return Err(D::Error::missing_field("value"));
            };

            if !config.ignore_unknown_fields && !obj.is_empty() {
                let unknown_key = obj
                    .keys()
                    .next()
                    .map(|key| key.as_str())
                    .unwrap_or("<no field>");
                return Err(D::Error::unknown_field(unknown_key, &["@type", "value"]));
            }

            value
        } else {
            JsonValue::Object(obj)
        };

        let mut res = Self {
            type_url,
            inner: Inner::Json(obj),
            cached: CACHED_INIT,
        };

        if has_type_resolver_set() {
            // Gracefully fail here and leave the `Self:;Json` variant in place.
            let _ = res.deserialize_any_in_place(Some(config));
        }

        Ok(res)
    }
}

#[derive(Debug, Clone)]
pub struct TypeRegistry {
    message_types: HashMap<String, AnyTypeDescriptor>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            message_types: HashMap::new(),
        }
    }

    pub fn new_with_well_known_types() -> Self {
        let mut registry = Self::new();
        registry.insert_default_well_known_types();
        registry
    }

    pub fn insert_default_well_known_types(&mut self) {
        self.insert_well_known_msg_type::<crate::any_v2::ProstAny>("google.protobuf.Any");
        self.insert_well_known_msg_type::<crate::protobuf::Timestamp>("google.protobuf.Timestamp");
        self.insert_well_known_msg_type::<crate::protobuf::Duration>("google.protobuf.Duration");
        self.insert_well_known_msg_type::<crate::protobuf::Struct>("google.protobuf.Struct");
        self.insert_well_known_msg_type::<f64>("google.protobuf.DoubleValue");
        self.insert_well_known_msg_type::<f32>("google.protobuf.FloatValue");
        self.insert_well_known_msg_type::<i64>("google.protobuf.Int64Value");
        self.insert_well_known_msg_type::<u64>("google.protobuf.UInt64Value");
        self.insert_well_known_msg_type::<i32>("google.protobuf.Int32Value");
        self.insert_well_known_msg_type::<u32>("google.protobuf.UInt32Value");
        self.insert_well_known_msg_type::<bool>("google.protobuf.BoolValue");
        self.insert_well_known_msg_type::<String>("google.protobuf.StringValue");
        self.insert_well_known_msg_type::<Vec<u8>>("google.protobuf.BytesValue");
        self.insert_well_known_msg_type::<Vec<u8>>("google.protobuf.BytesValue");
        self.insert_well_known_msg_type::<crate::protobuf::FieldMask>("google.protobuf.FieldMask");
        self.insert_well_known_msg_type::<crate::protobuf::ListValue>("google.protobuf.ListValue");
        self.insert_well_known_msg_type::<crate::protobuf::Value>("google.protobuf.Value");
        self.insert_well_known_msg_type::<()>("google.protobuf.Empty");
    }

    fn insert_well_known_msg_type<T>(&mut self, type_path: &str)
    where
        T: 'static + Message + SerdeMessage + Default + PartialEq + Clone,
    {
        let _ = self.message_types.insert(
            format!("type.googleapis.com/{type_path}"),
            AnyTypeDescriptor::for_type::<T>(),
        );
    }

    pub fn insert_msg_type<T>(&mut self)
    where
        T: 'static + Message + SerdeMessage + Name + Default + PartialEq + Clone,
    {
        let _ = self
            .message_types
            .insert(T::type_url(), AnyTypeDescriptor::for_type::<T>());
    }

    pub fn insert_msg_type_for_type_url<T>(&mut self, type_url: impl Into<String>)
    where
        T: 'static + Message + SerdeMessage + Default + PartialEq + Clone,
    {
        let _ = self
            .message_types
            .insert(type_url.into(), AnyTypeDescriptor::for_type::<T>());
    }

    pub fn remove_by_type_url(&mut self, type_url: &str) -> bool {
        self.message_types.remove(type_url).is_some()
    }

    pub fn into_type_resolver(self) -> Arc<dyn TypeResolver> {
        Arc::new(self) as _
    }
}

impl TypeResolver for TypeRegistry {
    fn resolve_message_type<'a>(&'a self, type_url: &str) -> Result<&'a AnyTypeDescriptor, ()> {
        self.message_types.get(type_url).ok_or(())
    }
}

#[derive(Debug, Clone)]
pub struct AnyTypeDescriptor {
    deserialize_protobuf: fn(&str, &[u8]) -> Result<Box<dyn AnyValue>, prost::DecodeError>,
    deserialize_json: fn(
        &str,
        &JsonValue,
        &prost::serde::DeserializerConfig,
    ) -> Result<Box<dyn AnyValue>, prost::DecodeError>,
}

impl AnyTypeDescriptor {
    // #[allow(private_bounds)]
    // pub fn for_well_known_type<T>() -> Self
    // where
    //     T: 'static + Message + Default + Clone,
    //     T: prost::serde::private::CustomSerialize,
    //     T: for<'de> prost::serde::private::CustomDeserialize<'de>,
    //     WellKnownWrapper<T>: AnyValue,
    // {
    //     fn deserialize_protobuf<T: 'static + Message + Default + Clone>(
    //         _type_url: &str,
    //         data: &[u8],
    //     ) -> Result<Box<dyn AnyValue>, prost::DecodeError>
    //     where
    //         WellKnownWrapper<T>: AnyValue,
    //     {
    //         Ok(Box::new(WellKnownWrapper(T::decode(data)?)) as _)
    //     }

    //     fn deserialize_json<T: 'static + Message + Default + Clone>(
    //         _type_url: &str,
    //         val: &JsonValue,
    //         config: &prost::serde::DeserializerConfig,
    //     ) -> Result<Box<dyn AnyValue>, prost::DecodeError>
    //     where
    //         WellKnownWrapper<T>: AnyValue,
    //         T: for<'de> prost::serde::private::CustomDeserialize<'de>,
    //     {
    //         let val = config
    //             .deserialize_from_value::<T>(val)
    //             .map_err(|err| prost::DecodeError::new(err.to_string()))?;

    //         Ok(Box::new(WellKnownWrapper(val.0)) as _)
    //     }

    //     Self {
    //         deserialize_protobuf: deserialize_protobuf::<T>,
    //         deserialize_json: deserialize_json::<T>,
    //     }
    // }

    pub fn for_type<T>() -> Self
    where
        T: 'static + Message + SerdeMessage + Default + PartialEq + Clone,
    {
        fn deserialize_protobuf<
            T: 'static + Message + SerdeMessage + Default + PartialEq + Clone,
        >(
            _type_url: &str,
            data: &[u8],
        ) -> Result<Box<dyn AnyValue>, prost::DecodeError> {
            Ok(Box::new(T::decode(data)?) as _)
        }

        fn deserialize_json<T: 'static + Message + SerdeMessage + Default + PartialEq + Clone>(
            _type_url: &str,
            val: &JsonValue,
            config: &prost::serde::DeserializerConfig,
        ) -> Result<Box<dyn AnyValue>, prost::DecodeError> {
            let val = config
                .deserialize_from_value::<T>(val)
                .map_err(|err| prost::DecodeError::new(err.to_string()))?;
            Ok(Box::new(val) as _)
        }

        Self {
            deserialize_protobuf: deserialize_protobuf::<T>,
            deserialize_json: deserialize_json::<T>,
        }
    }
}

pub fn default_type_resolver() -> Arc<dyn TypeResolver> {
    static DEFAULT_REGISTRY: OnceLock<Arc<TypeRegistry>> = OnceLock::new();
    DEFAULT_REGISTRY
        .get_or_init(|| Arc::new(TypeRegistry::new_with_well_known_types()))
        .clone()
}

pub trait TypeResolver {
    fn resolve_message_type<'a>(&'a self, type_url: &str) -> Result<&'a AnyTypeDescriptor, ()>;
}

thread_local! {
    static CURRENT_TYPE_RESOLVER: RefCell<Option<Arc<dyn TypeResolver>>> = RefCell::new(None);
}

pub fn with_type_resolver<F, R>(resolver: Option<Arc<dyn TypeResolver>>, f: F) -> R
where
    F: FnOnce() -> R,
{
    struct TypeResolverGuard(Option<Arc<dyn TypeResolver>>);
    impl Drop for TypeResolverGuard {
        fn drop(&mut self) {
            CURRENT_TYPE_RESOLVER.set(self.0.take());
        }
    }
    let _guard = TypeResolverGuard(CURRENT_TYPE_RESOLVER.replace(resolver));
    f()
}

pub fn with_default_type_resolver<R, F: FnOnce() -> R>(f: F) -> R {
    with_type_resolver(Some(default_type_resolver()), f)
}

fn has_type_resolver_set() -> bool {
    CURRENT_TYPE_RESOLVER.with_borrow(|type_resolver| type_resolver.is_some())
}

fn has_known_value_json_mapping(type_url: &str) -> bool {
    let Some(path) = type_url.strip_prefix("type.googleapis.com/") else {
        return false;
    };

    const KNOWN_PATHS: &[&str] = &[
        "google.protobuf.Any",
        "google.protobuf.Timestamp",
        "google.protobuf.Duration",
        "google.protobuf.Struct",
        "google.protobuf.DoubleValue",
        "google.protobuf.FloatValue",
        "google.protobuf.Int64Value",
        "google.protobuf.UInt64Value",
        "google.protobuf.Int32Value",
        "google.protobuf.UInt32Value",
        "google.protobuf.BoolValue",
        "google.protobuf.StringValue",
        "google.protobuf.BytesValue",
        "google.protobuf.FieldMask",
        "google.protobuf.ListValue",
        "google.protobuf.Value",
        "google.protobuf.Empty",
    ];

    KNOWN_PATHS.contains(&path)
}
