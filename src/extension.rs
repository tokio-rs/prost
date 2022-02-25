use crate::alloc::boxed::Box;
use crate::alloc::collections::btree_map::Entry;
use crate::alloc::collections::BTreeMap;
use crate::alloc::fmt::{Display, Formatter};
use crate::encoding::{DecodeContext, WireType};
use crate::generic::{EncodeBuffer, Merge, MergeBuffer, ProtoIntType};
use crate::{DecodeError, Encode};
use bytes::{Buf, BufMut};
use core::any::Any;
use core::fmt::Debug;
use core::marker::PhantomData;
use core::option::Option;

type ExtendableTypeId = &'static str;
type FieldTag = u32;

/// Marks a message as having able to have extensions.
/// Extension data must be retrieved and modified through use of static Extensions in generated code.
pub trait Extendable: 'static {
    /// A static type id associated with this Extendable.
    fn extendable_type_id() -> ExtendableTypeId;

    /// The data structure that stores extension data.
    fn extension_set(&self) -> &ExtensionSet<Self>;

    /// The data structure that stores extension data.
    fn extension_set_mut(&mut self) -> &mut ExtensionSet<Self>;

    /// Retrieve data using an existing Extension reference.
    ///
    /// Typically the Extension is a const value in your generated protobuf code.
    fn extension_data<T>(&self, extension: &ExtensionImpl<T>) -> Result<&T, ExtensionSetError>
    where
        T: 'static + Merge + Encode + Clone + PartialEq + Default + Debug,
    {
        self.extension_set().extension_data(extension)
    }

    /// Retrieve Extension data using an existing Extension reference.
    ///
    /// Typically the Extension is a const value in your generated protobuf code.
    fn extension_data_mut<T>(
        &mut self,
        extension: &ExtensionImpl<T>,
    ) -> Result<&mut T, ExtensionSetError>
    where
        T: 'static + Merge + Encode + Clone + PartialEq + Default + Debug,
    {
        self.extension_set_mut().extension_data_mut(extension)
    }

    /// Set Extension data using an existing Extension reference. If the value doesn't exist,
    /// it will be created with the new value.
    ///
    /// Typically the Extension is a const value in your generated protobuf code.
    fn set_extension_data<T>(
        &mut self,
        extension: &ExtensionImpl<T>,
        value: T,
    ) -> Result<(), ExtensionSetError>
    where
        T: 'static + Merge + Encode + Clone + PartialEq + Default + Debug,
    {
        self.extension_set_mut()
            .set_extension_data(extension, value)
    }

    /// Check if the this object has any data for the Extension.
    ///
    /// Typically the Extension is a const value in your generated protobuf code.
    fn has_extension(&self, extension: &dyn Extension) -> bool {
        self.extension_set().has_extension(extension)
    }

    /// Clear any data set for the Extension. If there was data, it will be returned.
    ///
    /// Typically the Extension is a const value in your generated protobuf code.
    fn clear_extension<T>(&mut self, extension: &ExtensionImpl<T>) -> Option<T>
    where
        T: 'static + Merge + Encode + Clone + PartialEq + Default + Debug,
    {
        self.extension_set_mut().clear_extension(extension)
    }
}

type ExtensionSetValues = BTreeMap<FieldTag, Box<dyn ExtensionValue>>;

/// Contains Extension data decoded using Extensions added to an ExtensionRegistry before decode.
///
/// To access or modify extension data you must have a reference to the same Extension that was used
/// to decode the data.
#[derive(Default)]
pub struct ExtensionSet<TOwner>
where
    TOwner: 'static + ?Sized,
{
    // Option<Box<_>> used to reduce size overhead in Extendable messages without any extensions.
    tag_to_value: Option<Box<ExtensionSetValues>>,

    _phantom: PhantomData<&'static TOwner>,
}

impl<TOwner> ExtensionSet<TOwner>
where
    TOwner: 'static + Extendable + ?Sized,
{
    pub fn extension_value<T>(
        &self,
        extension: &ExtensionImpl<T>,
    ) -> Result<&ExtensionValueImpl<T>, ExtensionSetError>
    where
        T: 'static + Merge + Encode + Clone + PartialEq + Default + Debug,
    {
        if extension.extendable_type_id() != TOwner::extendable_type_id() {
            return Err(ExtensionSetError::WrongExtendableTypeId);
        }
        let tag_to_value = match self.tag_to_value.as_ref() {
            None => return Err(ExtensionSetError::ExtensionNotFound),
            Some(val) => val,
        };
        let ext_value = match tag_to_value.get(&extension.field_tag()) {
            None => return Err(ExtensionSetError::ExtensionNotFound),
            Some(val) => val,
        };
        match ext_value.as_any().downcast_ref::<ExtensionValueImpl<T>>() {
            None => Err(ExtensionSetError::CastFailed),
            Some(val) => Ok(val),
        }
    }

    pub fn extension_value_mut<T>(
        &mut self,
        extension: &ExtensionImpl<T>,
    ) -> Result<&mut ExtensionValueImpl<T>, ExtensionSetError>
    where
        T: 'static + Merge + Encode + Clone + PartialEq + Default + Debug,
    {
        if extension.extendable_type_id() != TOwner::extendable_type_id() {
            return Err(ExtensionSetError::WrongExtendableTypeId);
        }
        let tag_to_value = match self.tag_to_value.as_mut() {
            None => return Err(ExtensionSetError::ExtensionNotFound),
            Some(val) => val,
        };
        let ext_value = match tag_to_value.get_mut(&extension.field_tag()) {
            None => return Err(ExtensionSetError::ExtensionNotFound),
            Some(val) => val,
        };
        match ext_value
            .as_any_mut()
            .downcast_mut::<ExtensionValueImpl<T>>()
        {
            None => Err(ExtensionSetError::CastFailed),
            Some(val) => Ok(val),
        }
    }

    pub fn extension_data<T>(&self, extension: &ExtensionImpl<T>) -> Result<&T, ExtensionSetError>
    where
        T: 'static + Merge + Encode + Clone + PartialEq + Default + Debug,
    {
        match self.extension_value(extension).map(|ext| ext.data()) {
            Ok(data) => data.ok_or(ExtensionSetError::ExtensionNotFound),
            Err(err) => Err(err),
        }
    }

    pub fn extension_data_mut<T>(
        &mut self,
        extension: &ExtensionImpl<T>,
    ) -> Result<&mut T, ExtensionSetError>
    where
        T: 'static + Merge + Encode + Clone + PartialEq + Default + Debug,
    {
        match self
            .extension_value_mut(extension)
            .map(|ext| ext.data_mut())
        {
            Ok(data) => data.ok_or(ExtensionSetError::ExtensionNotFound),
            Err(err) => Err(err),
        }
    }

    pub fn set_extension_data<T>(
        &mut self,
        extension: &ExtensionImpl<T>,
        value: T,
    ) -> Result<(), ExtensionSetError>
    where
        T: 'static + Merge + Encode + Clone + PartialEq + Default + Debug,
    {
        let ext_value = self.ext_value_or_new(extension);
        match ext_value
            .as_any_mut()
            .downcast_mut::<ExtensionValueImpl<T>>()
        {
            None => Err(ExtensionSetError::CastFailed),
            Some(val) => {
                val.set_data(value);
                Ok(())
            }
        }
    }

    pub fn has_extension(&self, extension: &dyn Extension) -> bool {
        self.tag_to_value
            .as_ref()
            .map(|values| {
                values
                    .get(&extension.field_tag())
                    .map(|value| value.has_data())
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    }

    pub fn clear_extension<T>(&mut self, extension: &ExtensionImpl<T>) -> Option<T>
    where
        T: 'static + Merge + Encode + Clone + PartialEq + Default + Debug,
    {
        let values = match self.tag_to_value.as_mut() {
            None => return None,
            Some(values) => values,
        };
        let mut value = match values.remove(&extension.field_tag) {
            None => return None,
            Some(value) => value,
        };
        match value.as_any_mut().downcast_mut::<ExtensionValueImpl<T>>() {
            None => None,
            Some(val) => val.take_data(),
        }
    }

    pub fn encode<B>(&self, buf: &mut B)
    where
        B: BufMut,
    {
        let mut encode_buffer = EncodeBuffer::new(buf);
        self.for_each_tag_value(|tag, value| {
            value.encode(value.proto_int_type(), *tag, &mut encode_buffer)
        });
    }

    pub fn encoded_len(&self) -> usize {
        let mut size = 0;
        self.for_each_tag_value(|tag, value| {
            size += value.encoded_len(value.proto_int_type(), *tag);
        });
        size
    }

    pub fn merge_field<B>(
        &mut self,
        tag: FieldTag,
        wire_type: WireType,
        mut buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
    {
        let registry = match ctx.extension_registry() {
            None => return crate::encoding::skip_field(wire_type, tag, buf, ctx),
            Some(registry) => registry,
        };
        let extension = match registry.extension(TOwner::extendable_type_id(), tag as FieldTag) {
            None => return crate::encoding::skip_field(wire_type, tag, buf, ctx),
            Some(extension) => extension,
        };
        let extension_value = self.ext_value_or_new(extension);
        let mut merge_buffer = MergeBuffer::new(&mut buf);
        extension_value.merge(
            extension.proto_int_type(),
            wire_type,
            &mut merge_buffer,
            ctx,
        )?;
        Ok(())
    }

    pub fn clear(&mut self) {
        self.tag_to_value = None;
    }

    fn ext_value_or_new(&mut self, extension: &dyn Extension) -> &mut Box<dyn ExtensionValue> {
        let tag = extension.field_tag();
        let tag_to_value = self
            .tag_to_value
            .get_or_insert_with(|| Box::new(ExtensionSetValues::default()));
        match tag_to_value.entry(tag) {
            Entry::Vacant(entry) => entry.insert(extension.create_value()),
            Entry::Occupied(entry) => entry.into_mut(),
        }
    }

    fn for_each_tag_value<F>(&self, mut action: F)
    where
        F: FnMut(&FieldTag, &Box<dyn ExtensionValue>),
    {
        let tag_to_value = match self.tag_to_value.as_ref() {
            None => return,
            Some(val) => val,
        };
        for (tag, value) in tag_to_value.iter() {
            action(tag, &value);
        }
    }
}

impl<TOwner> Clone for ExtensionSet<TOwner> {
    fn clone(&self) -> Self {
        let tag_to_value = match self.tag_to_value.as_ref() {
            None => None,
            Some(tag_to_value) => {
                let mut cloned = Box::new(ExtensionSetValues::default());
                for (tag, ext_value) in tag_to_value.as_ref() {
                    cloned.insert(*tag, ext_value.inner_clone());
                }
                Some(cloned)
            }
        };
        ExtensionSet {
            tag_to_value,
            _phantom: PhantomData {},
        }
    }
}

impl<TOwner> PartialEq for ExtensionSet<TOwner> {
    fn eq(&self, other: &Self) -> bool {
        let (lhs, rhs) = match (self.tag_to_value.as_ref(), other.tag_to_value.as_ref()) {
            (None, None) => return true,
            (Some(lhs), Some(rhs)) => (lhs.as_ref(), rhs.as_ref()),
            _ => return false,
        };
        if lhs.len() != rhs.len() {
            return false;
        }
        for (tag, value) in lhs {
            match rhs.get(tag) {
                None => return false,
                Some(other_value) => {
                    if !value.inner_eq(&**other_value) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

impl<TOwner> Debug for ExtensionSet<TOwner>
where
    TOwner: Extendable,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> alloc::fmt::Result {
        write!(
            f,
            "ExtensionSet({}): {:?}",
            TOwner::extendable_type_id(),
            self.tag_to_value
        )
    }
}

/// This error is provided as a convenience for debugging and testing instead of panicking. Only
/// the "normal case(s)" below should ever be encountered in a properly functioning application.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ExtensionSetError {
    /// Normal case. No value exists in the set with tag.
    ExtensionNotFound,

    /// User error. Attempting to retrieve a value using an Extension with a different target type.
    WrongExtendableTypeId,

    /// User error. Attempting to retrieve a value using a type parameter that does not match
    /// the stored data. This should probably never happen.
    CastFailed,
}

impl Display for ExtensionSetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> alloc::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Debug for ExtensionSetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> alloc::fmt::Result {
        match self {
            ExtensionSetError::ExtensionNotFound => write!(f, "Extension not found."),
            ExtensionSetError::WrongExtendableTypeId => {
                write!(f, "USER ERROR - Extension is for a different Extendable.")
            }
            ExtensionSetError::CastFailed => {
                write!(f, "USER ERROR - Type does not match stored data type.")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ExtensionSetError {}

/// A generic container for an ExtensionValue that can delegate to the implementation for type-specific work.
pub trait ExtensionValue: Merge + Encode + Any + Debug {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Forwards to inner type, allowing us to clone, just not through Clone trait.
    /// This works ok because dyn ExtensionValues should never be seen in user code.
    /// See `ExtensionSet`.
    fn inner_clone(&self) -> Box<dyn ExtensionValue>;

    /// Forwards to inner type, allowing us to compare, just not through PartialEq trait.
    /// This works ok because dyn ExtensionValues should never be seen in user code.
    /// See `ExtensionSet`.
    fn inner_eq(&self, other: &dyn ExtensionValue) -> bool;

    /// True if the inner type is considered to have data.
    fn has_data(&self) -> bool;

    /// ProtoIntType of the underlying type, cached for encoding.
    fn proto_int_type(&self) -> ProtoIntType;
}

/// A concrete holder for strongly typed extension data.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExtensionValueImpl<T> {
    // This is an Option so the data can be moved out from a &mut.
    data: Option<T>,

    proto_int_type: ProtoIntType,
}

impl<T> ExtensionValueImpl<T> {
    pub fn data(&self) -> Option<&T> {
        self.data.as_ref()
    }
    pub fn data_mut(&mut self) -> Option<&mut T> {
        self.data.as_mut()
    }
    pub fn set_data(&mut self, value: T) {
        self.data = Some(value);
    }
    pub fn take_data(&mut self) -> Option<T> {
        self.data.take()
    }
}

impl<T> Merge for ExtensionValueImpl<T>
where
    T: Merge + Default,
{
    fn merge(
        &mut self,
        proto_int_type: ProtoIntType,
        wire_type: WireType,
        buf: &mut MergeBuffer,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError> {
        let data = self.data.get_or_insert_with(T::default);
        data.merge(proto_int_type, wire_type, buf, ctx)
    }
}

impl<T> Encode for ExtensionValueImpl<T>
where
    T: Encode,
{
    fn encode(&self, proto_int_type: ProtoIntType, tag: u32, buf: &mut EncodeBuffer) {
        if let Some(data) = &self.data {
            data.encode(proto_int_type, tag, buf);
        }
    }

    fn encoded_len(&self, proto_int_type: ProtoIntType, tag: u32) -> usize {
        match &self.data {
            None => 0,
            Some(data) => data.encoded_len(proto_int_type, tag),
        }
    }
}

impl<T> ExtensionValue for ExtensionValueImpl<T>
where
    T: 'static + Merge + Encode + Clone + PartialEq + Default + Debug,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn inner_clone(&self) -> Box<dyn ExtensionValue> {
        let cloned = (*self).clone();
        Box::new(cloned)
    }

    fn inner_eq(&self, other: &dyn ExtensionValue) -> bool {
        let any = other.as_any().downcast_ref::<ExtensionValueImpl<T>>();
        match any {
            None => false,
            Some(other) => self == other,
        }
    }

    fn has_data(&self) -> bool {
        self.data.is_some()
    }

    fn proto_int_type(&self) -> ProtoIntType {
        self.proto_int_type
    }
}

/// A generic container for an Extension that can delegate to the implementation for type-specific work.
pub trait Extension {
    /// Fully-qualified type name of the message this extension is for.
    fn extendable_type_id(&self) -> ExtendableTypeId;

    /// Field number tag that this extension can be found at on the target message.
    fn field_tag(&self) -> FieldTag;

    /// Additional information on the type of integer the data is (if any).
    fn proto_int_type(&self) -> ProtoIntType;

    /// Create an instance of a concrete ExtensionValueImpl boxed as a generic ExtensionValue.
    fn create_value(&self) -> Box<dyn ExtensionValue>;
}

impl Debug for dyn Extension {
    fn fmt(&self, f: &mut Formatter<'_>) -> alloc::fmt::Result {
        write!(
            f,
            "{{field_tag: {:?}, extendable_type_id: {:?}}}",
            self.field_tag(),
            self.extendable_type_id()
        )
    }
}

/// A concrete implementation of an Extension that tracks type information of the contained data.
pub struct ExtensionImpl<T> {
    pub extendable_type_id: ExtendableTypeId,
    pub field_tag: FieldTag,
    pub proto_int_type: ProtoIntType,

    // Concrete impl tracks the value type T with phantom data, which can be used to
    // create the concrete ExtensionValueImpl type when requested.
    pub _phantom: PhantomData<*const T>,
}

impl<T> Extension for ExtensionImpl<T>
where
    T: 'static + Merge + Encode + Clone + PartialEq + Default + Debug,
{
    fn extendable_type_id(&self) -> ExtendableTypeId {
        self.extendable_type_id
    }

    fn field_tag(&self) -> FieldTag {
        self.field_tag
    }

    fn proto_int_type(&self) -> ProtoIntType {
        self.proto_int_type
    }

    fn create_value(&self) -> Box<dyn ExtensionValue> {
        Box::new(ExtensionValueImpl::<T> {
            proto_int_type: self.proto_int_type,
            ..Default::default()
        })
    }
}

type RegistryKey = (ExtendableTypeId, FieldTag);

/// A runtime container for generic extensions that should be used for decoding.
///
/// Users should load the registry with the static Extensions from generated code via `register`
/// before decoding a message.
#[derive(Default)]
pub struct ExtensionRegistry {
    extensions: BTreeMap<RegistryKey, &'static dyn Extension>,
}

impl Debug for ExtensionRegistry {
    fn fmt(&self, f: &mut Formatter<'_>) -> alloc::fmt::Result {
        write!(f, "ExtensionRegistry(size: {})", self.extensions.len())
    }
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn register(&mut self, extension: &'static dyn Extension) {
        self.extensions.insert(registry_key(extension), extension);
    }

    pub fn extension(
        &self,
        type_id: ExtendableTypeId,
        tag: FieldTag,
    ) -> Option<&'static dyn Extension> {
        self.extensions.get(&(type_id, tag)).copied()
    }
}

fn registry_key(extension: &'static dyn Extension) -> RegistryKey {
    (extension.extendable_type_id(), extension.field_tag())
}

#[cfg(test)]
mod tests {
    use crate::alloc::boxed::Box;
    use crate::alloc::string::{String, ToString};
    use crate::extension::{ExtendableTypeId, ExtensionValueImpl};
    use crate::{Extendable, ExtensionImpl, ExtensionSet, ExtensionValue, ProtoIntType};
    use core::marker::PhantomData;

    mod extension_value {
        use crate::alloc::string::{String, ToString};
        use crate::extension::tests::{create_ext_value, create_ext_value_dyn};
        use crate::extension::ExtensionValueImpl;

        #[test]
        fn partial_eq() {
            let a = create_ext_value("hello");
            let b = create_ext_value("hello");
            let c = create_ext_value("goodbye");
            assert_eq!(a, a);
            assert_eq!(a, b);
            assert_ne!(a, c);
            assert_ne!(b, c);
        }

        #[test]
        fn partial_eq_erased() {
            let a = create_ext_value_dyn("hello");
            let b = create_ext_value_dyn("hello");
            let c = create_ext_value_dyn("goodbye");
            assert!(a.inner_eq(&*a));
            assert!(a.inner_eq(&*b));
            assert!(!a.inner_eq(&*c));
            assert!(!b.inner_eq(&*c));
        }

        #[test]
        fn clone() {
            let value = create_ext_value("hello");
            let cloned = value.clone();
            assert_eq!(value, cloned);
            assert_eq!(value.data, cloned.data);
        }

        #[test]
        fn clone_erased() {
            let value = create_ext_value_dyn("hello");
            let cloned = value.inner_clone();
            assert_eq!(
                value
                    .as_any()
                    .downcast_ref::<ExtensionValueImpl<String>>()
                    .unwrap()
                    .data,
                Some("hello".to_string())
            );
            assert_eq!(
                cloned
                    .as_any()
                    .downcast_ref::<ExtensionValueImpl<String>>()
                    .unwrap()
                    .data,
                Some("hello".to_string())
            );
            assert!(value.inner_eq(&*cloned));
        }
    }

    mod extension_set {
        use crate::alloc::boxed::Box;
        use crate::alloc::string::ToString;
        use crate::extension::tests::{
            create_ext_value, TestExtendable, TEST_EXTENSION, TEST_EXTENSION_B,
        };
        use crate::extension::{ExtensionSetError, ExtensionSetValues};
        use crate::{Extension, ExtensionSet, ExtensionValue};

        #[test]
        fn partial_eq() {
            let a = create_test_extension_set("hello");
            let b = create_test_extension_set("hello");
            let c = create_test_extension_set("goodbye");
            assert_eq!(a, a);
            assert_eq!(a, b);
            assert_ne!(a, c);
            assert_ne!(b, c);
        }

        #[test]
        fn clone() {
            let extension_set = create_test_extension_set("data");
            let ext_value = Box::new(create_ext_value("data"));
            let cloned_set = extension_set.clone();
            let cloned_value = cloned_set
                .tag_to_value
                .as_ref()
                .expect("Cloned set has no extension values")
                .get(&TEST_EXTENSION.field_tag())
                .expect("Cloned set does not have the extension value.");
            assert!(ext_value.inner_eq(&**cloned_value));
        }

        mod extension_data {
            use crate::alloc::string::ToString;
            use crate::extension::tests::extension_set::create_test_extension_set;
            use crate::extension::tests::{OTHER_TEST_EXTENSION, TEST_EXTENSION, TEST_EXTENSION_B};
            use crate::extension::ExtensionSetError;

            #[test]
            fn happy_path() {
                let value = "data".to_string();
                let extension_set = create_test_extension_set(&value);
                assert_eq!(extension_set.extension_data(&TEST_EXTENSION), Ok(&value));
            }

            #[test]
            fn happy_path_mut() {
                let value = "data".to_string();
                let mut extension_set = create_test_extension_set(&value);
                let value_mut = extension_set
                    .extension_data_mut(&TEST_EXTENSION)
                    .expect("Failed to get ext data mut");
                assert_eq!(value_mut, &value);
                let new_value = "new_data".to_string();
                *value_mut = new_value.clone();
                assert_eq!(
                    extension_set.extension_data(&TEST_EXTENSION),
                    Ok(&new_value)
                );
            }

            #[test]
            fn wrong_type_id() {
                let extension_set = create_test_extension_set("data");
                assert_eq!(
                    extension_set.extension_data(&OTHER_TEST_EXTENSION),
                    Err(ExtensionSetError::WrongExtendableTypeId)
                );
            }

            #[test]
            fn invalid_cast() {
                // It's not actually possible to cause this case due to extension_value method generics.
            }

            #[test]
            fn normal_case_no_value() {
                let extension_set = create_test_extension_set("data");
                assert_eq!(
                    extension_set.extension_data(&TEST_EXTENSION_B),
                    Err(ExtensionSetError::ExtensionNotFound)
                );
            }
        }

        #[test]
        fn test_set_extension_data() {
            let mut extension_set = create_test_extension_set("data");
            let new_value = "new_value".to_string();
            extension_set
                .set_extension_data(&TEST_EXTENSION, new_value.clone())
                .expect("Failed to set ext data");
            assert_eq!(
                extension_set.extension_data(&TEST_EXTENSION),
                Ok(&new_value)
            );
        }

        #[test]
        fn test_clear_extension_data() {
            let value = "data".to_string();
            let other_value = 12345;
            let mut extension_set = create_test_extension_set(&value);
            assert_eq!(extension_set.extension_data(&TEST_EXTENSION), Ok(&value));
            extension_set
                .set_extension_data(&TEST_EXTENSION_B, other_value)
                .expect("Failed to set ext data");
            extension_set.clear_extension(&TEST_EXTENSION);
            assert!(
                !extension_set.has_extension(&TEST_EXTENSION),
                "Should clear targeted data"
            );
            assert_eq!(
                extension_set.extension_data(&TEST_EXTENSION_B),
                Ok(&other_value),
                "Should not clear other data in the ext set"
            );
        }

        #[test]
        fn test_take_extension_data() -> Result<(), ExtensionSetError> {
            let data = "data".to_string();
            let mut extension_set = create_test_extension_set(&data);
            let taken_data = extension_set
                .extension_value_mut(&TEST_EXTENSION)?
                .take_data();
            assert_eq!(taken_data, Some(data));
            assert!(!extension_set.has_extension(&TEST_EXTENSION));
            Ok(())
        }

        fn create_test_extension_set(ext_value_data: &str) -> ExtensionSet<TestExtendable> {
            let mut extension_set = ExtensionSet::<TestExtendable>::default();
            let ext_value = Box::new(create_ext_value(ext_value_data));
            extension_set.tag_to_value = Some(Box::new(ExtensionSetValues::default()));
            extension_set
                .tag_to_value
                .as_mut()
                .unwrap()
                .insert(TEST_EXTENSION.field_tag(), ext_value.inner_clone());
            extension_set
        }
    }

    fn create_ext_value_dyn(data: impl ToString) -> Box<dyn ExtensionValue> {
        Box::new(create_ext_value(data))
    }

    fn create_ext_value(data: impl ToString) -> ExtensionValueImpl<String> {
        ExtensionValueImpl::<String> {
            data: Some(data.to_string()),
            proto_int_type: Default::default(),
        }
    }

    const TEST_EXTENDABLE_ID: &str = "TestExtendable";
    const TEST_EXTENSION: ExtensionImpl<String> = ExtensionImpl::<String> {
        extendable_type_id: TEST_EXTENDABLE_ID,
        field_tag: 50000,
        proto_int_type: ProtoIntType::Default,
        _phantom: PhantomData {},
    };
    const TEST_EXTENSION_B: ExtensionImpl<i32> = ExtensionImpl::<i32> {
        extendable_type_id: TEST_EXTENDABLE_ID,
        field_tag: 50001,
        proto_int_type: ProtoIntType::Default,
        _phantom: PhantomData {},
    };
    const OTHER_TEST_EXTENSION: ExtensionImpl<String> = ExtensionImpl::<String> {
        extendable_type_id: "OtherTestExtendable",
        field_tag: 50001,
        proto_int_type: ProtoIntType::Default,
        _phantom: PhantomData {},
    };

    #[derive(Default)]
    struct TestExtendable {
        pub extension_set: ExtensionSet<TestExtendable>,
    }
    impl Extendable for TestExtendable {
        fn extendable_type_id() -> ExtendableTypeId {
            TEST_EXTENDABLE_ID
        }

        fn extension_set(&self) -> &ExtensionSet<Self> {
            &self.extension_set
        }

        fn extension_set_mut(&mut self) -> &mut ExtensionSet<Self> {
            &mut self.extension_set
        }
    }
}
