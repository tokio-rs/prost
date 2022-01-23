//! Tests custom proto options.

use prost::alloc::string::String;
use prost::{Extendable, Extension, ExtensionRegistry, Message};
use prost_types::{
    DescriptorProto, EnumDescriptorProto, FileDescriptorSet, FileOptions, MessageOptions,
};

mod custom_options {
    include!(concat!(env!("OUT_DIR"), "/custom_options.rs"));
}
mod custom_options_ext {
    include!(concat!(env!("OUT_DIR"), "/custom_options.ext.rs"));
}

const DESCRIPTOR_SET_BYTES: &'static [u8] = include_bytes!(concat!(
    env!("OUT_DIR"),
    "/custom_options_descriptor_set.bin"
));

macro_rules! test_custom_option {
    ($test_name: ident, $extension: expr, $expected: expr) => {
        #[test]
        fn $test_name() {
            run_test(&$extension, $expected)
        }
    };
}

// File options - tests every type to ensure parsing.
mod file {
    use alloc::format;
    use alloc::string::ToString;
    use core::fmt::Debug;

    use prost::{Encode, Extendable, ExtensionImpl, Merge};

    use super::{custom_options, find_test_file_options, load_desc_set};

    // Non-repeated.
    test_custom_option!(double, custom_options::FILE_OPT_DOUBLE, 12345.6);
    test_custom_option!(float, custom_options::FILE_OPT_FLOAT, 12345.6);
    test_custom_option!(int32, custom_options::FILE_OPT_INT32, -12345);
    test_custom_option!(int64, custom_options::FILE_OPT_INT64, -12345);
    test_custom_option!(uint32, custom_options::FILE_OPT_UINT32, 12345);
    test_custom_option!(uint64, custom_options::FILE_OPT_UINT64, 12345);
    test_custom_option!(sint32, custom_options::FILE_OPT_SINT32, -12345);
    test_custom_option!(sint64, custom_options::FILE_OPT_SINT64, -12345);
    test_custom_option!(fixed32, custom_options::FILE_OPT_FIXED32, 12345);
    test_custom_option!(fixed64, custom_options::FILE_OPT_FIXED64, 12345);
    test_custom_option!(sfixed32, custom_options::FILE_OPT_SFIXED32, -12345);
    test_custom_option!(sfixed64, custom_options::FILE_OPT_SFIXED64, -12345);
    test_custom_option!(bool, custom_options::FILE_OPT_BOOL, true);
    test_custom_option!(string, custom_options::FILE_OPT_STRING, "hello".to_string());
    test_custom_option!(
        bytes,
        custom_options::FILE_OPT_BYTES,
        "hello".to_string().into_bytes()
    );
    test_custom_option!(
        message,
        custom_options::FILE_OPT_MESSAGE,
        custom_options::CustomOptionType {
            s: "hello".to_string(),
            i: -12345,
        }
    );

    // Repeated.
    test_custom_option!(
        repeated_double,
        custom_options::FILE_OPT_DOUBLE_REPEATED,
        vec![11111.1, 55555.5]
    );
    test_custom_option!(
        repeated_float,
        custom_options::FILE_OPT_FLOAT_REPEATED,
        vec![11111.1, 55555.5]
    );
    test_custom_option!(
        repeated_int32,
        custom_options::FILE_OPT_INT32_REPEATED,
        vec![-11111, 55555]
    );
    test_custom_option!(
        repeated_int64,
        custom_options::FILE_OPT_INT64_REPEATED,
        vec![-11111, 55555]
    );
    test_custom_option!(
        repeated_uint32,
        custom_options::FILE_OPT_UINT32_REPEATED,
        vec![11111, 55555]
    );
    test_custom_option!(
        repeated_uint64,
        custom_options::FILE_OPT_UINT64_REPEATED,
        vec![11111, 55555]
    );
    test_custom_option!(
        repeated_sint32,
        custom_options::FILE_OPT_SINT32_REPEATED,
        vec![-11111, 55555]
    );
    test_custom_option!(
        repeated_sint64,
        custom_options::FILE_OPT_SINT64_REPEATED,
        vec![-11111, 55555]
    );
    test_custom_option!(
        repeated_fixed32,
        custom_options::FILE_OPT_FIXED32_REPEATED,
        vec![11111, 55555]
    );
    test_custom_option!(
        repeated_fixed64,
        custom_options::FILE_OPT_FIXED64_REPEATED,
        vec![11111, 55555]
    );
    test_custom_option!(
        repeated_sfixed32,
        custom_options::FILE_OPT_SFIXED32_REPEATED,
        vec![-11111, 55555]
    );
    test_custom_option!(
        repeated_sfixed64,
        custom_options::FILE_OPT_SFIXED64_REPEATED,
        vec![-11111, 55555]
    );
    test_custom_option!(
        repeated_bool,
        custom_options::FILE_OPT_BOOL_REPEATED,
        vec![true, false]
    );
    test_custom_option!(
        repeated_string,
        custom_options::FILE_OPT_STRING_REPEATED,
        vec!["hello".to_string(), "goodbye".to_string()]
    );
    test_custom_option!(
        repeated_bytes,
        custom_options::FILE_OPT_BYTES_REPEATED,
        vec![
            "hello".to_string().into_bytes(),
            "goodbye".to_string().into_bytes()
        ]
    );
    test_custom_option!(
        repeated_message,
        custom_options::FILE_OPT_MESSAGE_REPEATED,
        vec![
            custom_options::CustomOptionType {
                s: "hello".to_string(),
                i: -11111,
            },
            custom_options::CustomOptionType {
                s: "goodbye".to_string(),
                i: 55555,
            },
        ]
    );

    fn run_test<T>(extension: &'static ExtensionImpl<T>, expected: T)
    where
        T: 'static + Merge + Encode + Default + Debug + PartialEq + Clone,
    {
        let file_descriptor_set = load_desc_set(extension);
        let file_options = find_test_file_options(&file_descriptor_set);
        let ext_data = file_options.extension_data(extension).expect(&format!(
            "Extension has incorrect data, expected: {:?}",
            expected
        ));
        // Data specified in custom_options.proto.
        assert_eq!(ext_data, &expected);
    }
}

mod message {
    use alloc::format;
    use alloc::string::ToString;
    use core::fmt::Debug;

    use prost::{Encode, Extendable, ExtensionImpl, Merge};

    use super::{custom_options, custom_options_ext, find_test_msg_options, load_desc_set};

    // Message options.
    test_custom_option!(string, custom_options::MESSAGE_OPT, "hello".to_string());

    // Nested options.
    test_custom_option!(
        nested_string,
        custom_options::NestedOptions::NESTED_OPT,
        "hello".to_string()
    );

    // Option from another file.
    test_custom_option!(
        ext_string,
        custom_options_ext::MESSAGE_OPT_FROM_EXT,
        "hello".to_string()
    );

    // Nested option from another file.
    test_custom_option!(
        ext_nested_string,
        custom_options_ext::NestedOptions::NESTED_OPT_FROM_EXT,
        "hello".to_string()
    );

    fn run_test<T>(extension: &'static ExtensionImpl<T>, expected: T)
    where
        T: 'static + Merge + Encode + Default + Debug + PartialEq + Clone,
    {
        let file_descriptor_set = load_desc_set(extension);
        let message_options = find_test_msg_options(&file_descriptor_set);
        let ext_data = message_options.extension_data(extension).expect(&format!(
            "Extension has incorrect data, expected: {:?}",
            expected
        ));
        // Data specified in custom_options.proto.
        assert_eq!(ext_data, &expected);
    }
}

#[test]
fn field() {
    let extension = &custom_options::FIELD_OPT;
    let file_descriptor_set = load_desc_set(extension);
    let test_msg = find_test_msg(&file_descriptor_set);
    let field = test_msg
        .field
        .iter()
        .find(|f| f.name.as_ref().unwrap() == "field_with_opt")
        .expect("Missing field 'field_with_opt'.");
    let field_options = field.options.as_ref().expect("Field has no options");
    let ext_data = field_options
        .extension_data(extension)
        .expect("Extension data missing.");
    // Data specified in custom_options.proto.
    assert_eq!(ext_data, &"hello");
}

#[test]
fn oneof() {
    let extension = &custom_options::ONEOF_OPT;
    let file_descriptor_set = load_desc_set(extension);
    let test_msg = find_test_msg(&file_descriptor_set);
    assert_eq!(test_msg.oneof_decl.len(), 1);
    let oneof = test_msg.oneof_decl.get(0).expect("OneOf missing");
    let oneof_options = oneof.options.as_ref().expect("OneOf has no options");
    let ext_data = oneof_options
        .extension_data(extension)
        .expect("Extension data missing");
    // Data specified in custom_options.proto.
    assert_eq!(ext_data, &"hello");
}

#[test]
fn enum_opt() {
    let extension = &custom_options::ENUM_OPT;
    let file_descriptor_set = load_desc_set(extension);
    let test_enum = find_test_enum(&file_descriptor_set);
    let enum_options = test_enum.options.as_ref().expect("Enum has no options");
    let ext_data = enum_options
        .extension_data(extension)
        .expect("Extension data missing");
    // Data specified in custom_options.proto.
    assert_eq!(ext_data, &"hello");
}

#[test]
fn enum_value() {
    let extension = &custom_options::ENUM_VALUE_OPT;
    let file_descriptor_set = load_desc_set(extension);
    let test_enum = find_test_enum(&file_descriptor_set);
    let enum_value = test_enum.value.get(0).expect("Enum value missing.");
    assert_eq!(enum_value.name.as_ref().unwrap(), "Default");
    let value_options = enum_value
        .options
        .as_ref()
        .expect("Enum value has no options");
    let ext_data = value_options
        .extension_data(extension)
        .expect("Extension data missing");
    // Data specified in custom_options.proto.
    assert_eq!(ext_data, &"hello");
}

fn load_desc_set(extension_to_test: &'static dyn Extension) -> FileDescriptorSet {
    Message::decode_with_extensions(DESCRIPTOR_SET_BYTES, registry(extension_to_test))
        .expect("Failed to decode descriptor set")
}

fn registry(extension_to_test: &'static dyn Extension) -> ExtensionRegistry {
    let mut extension_registry = ExtensionRegistry::new();
    extension_registry.register(extension_to_test);
    extension_registry
}

fn find_test_file_options(descriptor_set: &FileDescriptorSet) -> &FileOptions {
    for file in &descriptor_set.file {
        if file.name.as_ref().unwrap() == "custom_options.proto" {
            return file.options.as_ref().expect("File does not have options.");
        }
    }
    panic!("Could not find test options file.");
}

fn find_test_msg_options(descriptor_set: &FileDescriptorSet) -> &MessageOptions {
    find_test_msg(descriptor_set)
        .options
        .as_ref()
        .expect("Message does not have options.")
}

fn find_test_msg(descriptor_set: &FileDescriptorSet) -> &DescriptorProto {
    for file in &descriptor_set.file {
        match file.message_type.iter().find(|proto| {
            proto
                .name
                .as_ref()
                .expect("Message has no name")
                .contains("MessageWithCustomOptions")
        }) {
            None => continue,
            Some(proto) => return proto,
        }
    }
    panic!("Could not find MessageWithCustomOptions test message.");
}

fn find_test_enum(descriptor_set: &FileDescriptorSet) -> &EnumDescriptorProto {
    for file in &descriptor_set.file {
        match file.enum_type.iter().find(|proto| {
            proto
                .name
                .as_ref()
                .expect("Enum has no name")
                .contains("EnumWithCustomOptions")
        }) {
            None => continue,
            Some(proto) => return proto,
        }
    }
    panic!("Could not find EnumWithCustomOptions test enum.");
}
