include!(concat!(env!("OUT_DIR"), "/type_name_prefix.rs"));

use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec::Vec;

#[test]
fn test_type_names_have_prefix() {
    // Create instances with the prefixed names
    let _ = ProtoTestMessage {
        name: "test".to_string(),
        value: 42,
    };

    // Test that we can use the enum with prefix
    let test_enum = ProtoTestEnum::First;

    // Test nested types have prefix
    let _ = ProtoOuterMessage {
        inner: Some(outer_message::ProtoInnerMessage {
            data: "inner data".to_string(),
        }),
        test_enum: test_enum as i32,
    };

    // Test self-referencing types work with prefix
    let _ = ProtoRecursiveMessage {
        child: Some(Box::new(ProtoRecursiveMessage {
            child: None,
            children: Vec::new(),
        })),
        children: Vec::from([ProtoRecursiveMessage {
            child: None,
            children: Vec::new(),
        }]),
    };
}

#[test]
fn test_oneof_with_prefix() {
    use self::message_with_oneof::ProtoTestOneof;

    // Test that we can create oneof variants with the prefixed enum name
    let _ = ProtoMessageWithOneof {
        test_oneof: Some(ProtoTestOneof::StringValue("test".to_string())),
    };

    let _ = ProtoMessageWithOneof {
        test_oneof: Some(ProtoTestOneof::IntValue(42)),
    };

    let _ = ProtoMessageWithOneof {
        test_oneof: Some(ProtoTestOneof::Message(ProtoTestMessage {
            name: "nested".to_string(),
            value: 100,
        })),
    };
}
