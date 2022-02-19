//! Tests proto2 extensions.

use alloc::string::ToString;
use alloc::vec::Vec;
use prost::{DecodeError, Extendable, Extension, ExtensionRegistry, Message};

mod extensions {
    include!(concat!(env!("OUT_DIR"), "/extensions.rs"));
}

macro_rules! test_extension {
    ($test_name: ident, $extension: expr, $value: expr) => {
        #[test]
        fn $test_name() -> Result<(), DecodeError> {
            let mut message = extensions::ExtendableMessage::default();
            message
                .set_extension_data($extension, $value)
                .expect("Failed to set ext data");
            let message = roundtrip(&message, $extension);
            let ext_data = message
                .extension_data($extension)
                .expect("Failed to get ext data");
            assert_eq!(*ext_data, $value);
            Ok(())
        }
    };
}

// Non-repeated.
test_extension!(double, extensions::EXT_DOUBLE, 12345.6);
test_extension!(float, extensions::EXT_FLOAT, 12345.6);
test_extension!(int32, extensions::EXT_INT32, -12345);
test_extension!(int64, extensions::EXT_INT64, -12345);
test_extension!(uint32, extensions::EXT_UINT32, 12345);
test_extension!(uint64, extensions::EXT_UINT64, 12345);
test_extension!(sint32, extensions::EXT_SINT32, -12345);
test_extension!(sint64, extensions::EXT_SINT64, -12345);
test_extension!(fixed32, extensions::EXT_FIXED32, 12345);
test_extension!(fixed64, extensions::EXT_FIXED64, 12345);
test_extension!(sfixed32, extensions::EXT_SFIXED32, -12345);
test_extension!(sfixed64, extensions::EXT_SFIXED64, -12345);
test_extension!(bool, extensions::EXT_BOOL, true);
test_extension!(string, extensions::EXT_STRING, "hello".to_string());
test_extension!(
    bytes,
    extensions::EXT_BYTES,
    "hello".to_string().into_bytes()
);
test_extension!(
    message,
    extensions::EXT_MESSAGE,
    extensions::CustomMessageType {
        some_value: Some("hello".to_string())
    }
);

// Repeated.
test_extension!(
    repeated_double,
    extensions::EXT_DOUBLE_REPEATED,
    vec![11111.1, 55555.5]
);
test_extension!(
    repeated_float,
    extensions::EXT_FLOAT_REPEATED,
    vec![11111.1, 55555.5]
);
test_extension!(
    repeated_int32,
    extensions::EXT_INT32_REPEATED,
    vec![-11111, 55555]
);
test_extension!(
    repeated_int64,
    extensions::EXT_INT64_REPEATED,
    vec![-11111, 55555]
);
test_extension!(
    repeated_uint32,
    extensions::EXT_UINT32_REPEATED,
    vec![11111, 55555]
);
test_extension!(
    repeated_uint64,
    extensions::EXT_UINT64_REPEATED,
    vec![11111, 55555]
);
test_extension!(
    repeated_sint32,
    extensions::EXT_SINT32_REPEATED,
    vec![-11111, 55555]
);
test_extension!(
    repeated_sint64,
    extensions::EXT_SINT64_REPEATED,
    vec![-11111, 55555]
);
test_extension!(
    repeated_fixed32,
    extensions::EXT_FIXED32_REPEATED,
    vec![11111, 55555]
);
test_extension!(
    repeated_fixed64,
    extensions::EXT_FIXED64_REPEATED,
    vec![11111, 55555]
);
test_extension!(
    repeated_sfixed32,
    extensions::EXT_SFIXED32_REPEATED,
    vec![-11111, 55555]
);
test_extension!(
    repeated_sfixed64,
    extensions::EXT_SFIXED64_REPEATED,
    vec![-11111, 55555]
);
test_extension!(
    repeated_bool,
    extensions::EXT_BOOL_REPEATED,
    vec![true, false]
);
test_extension!(
    repeated_string,
    extensions::EXT_STRING_REPEATED,
    vec!["hello".to_string(), "goodbye".to_string()]
);
test_extension!(
    repeated_bytes,
    extensions::EXT_BYTES_REPEATED,
    vec![
        "hello".to_string().into_bytes(),
        "goodbye".to_string().into_bytes()
    ]
);
test_extension!(
    repeated_message,
    extensions::EXT_MESSAGE_REPEATED,
    vec![
        extensions::CustomMessageType {
            some_value: Some("hello".to_string()),
        },
        extensions::CustomMessageType {
            some_value: Some("goodbye".to_string()),
        },
    ]
);

#[test]
fn clear_message_clears_ext_set() {
    let mut message = extensions::ExtendableMessage::default();
    let data = "data".to_string();
    message
        .set_extension_data(extensions::EXT_STRING, data.clone())
        .expect("Failed to set ext data");
    assert_eq!(message.extension_data(extensions::EXT_STRING), Ok(&data));
    message.clear();
    assert!(message.extension_data(extensions::EXT_STRING).is_err());
}

#[test]
fn register_extensions() {
    let mut extension_registry = ExtensionRegistry::new();
    extensions::register_extensions(&mut extension_registry);

    let enum_value_opt = extension_registry.extension(
        extensions::EXT_MESSAGE_REPEATED.extendable_type_id(),
        extensions::EXT_MESSAGE_REPEATED.field_tag(),
    );
    assert!(matches!(enum_value_opt, Some(_)));
}

fn roundtrip<M>(message: &M, extension: &'static dyn Extension) -> M
where
    M: Message + Default,
{
    let mut buf = Vec::new();
    buf.reserve(message.encoded_len());
    message.encode(&mut buf).expect("Failed to encode.");
    M::decode_with_extensions(&mut buf.as_slice(), registry(extension)).expect("Failed to decode.")
}

fn registry(extension: &'static dyn Extension) -> ExtensionRegistry {
    let mut registry = ExtensionRegistry::new();
    registry.register(extension);
    registry
}
