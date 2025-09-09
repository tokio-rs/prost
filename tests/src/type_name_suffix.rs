include!(concat!(env!("OUT_DIR"), "/type_name_suffix.rs"));

use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec::Vec;

#[test]
fn test_type_names_have_suffix() {
    // Create instances with the suffixed names
    let _ = TestMessageProto {
        name: "test".to_string(),
        value: 42,
    };

    // Test that we can use the enum with suffix
    let test_enum = TestEnumProto::First;

    // Test nested types have suffix
    let _ = OuterMessageProto {
        inner: Some(outer_message::InnerMessageProto {
            data: "inner data".to_string(),
        }),
        test_enum: test_enum as i32,
    };

    // Test self-referencing types work with suffix
    let _ = RecursiveMessageProto {
        child: Some(Box::new(RecursiveMessageProto {
            child: None,
            children: Vec::new(),
        })),
        children: Vec::from([RecursiveMessageProto {
            child: None,
            children: Vec::new(),
        }]),
    };
}

#[test]
fn test_oneof_with_suffix() {
    use self::message_with_oneof::TestOneofProto;

    // Test that we can create oneof variants with the suffixed enum name
    let _ = MessageWithOneofProto {
        test_oneof: Some(TestOneofProto::StringValue("test".to_string())),
    };

    let _ = MessageWithOneofProto {
        test_oneof: Some(TestOneofProto::IntValue(42)),
    };

    let _ = MessageWithOneofProto {
        test_oneof: Some(TestOneofProto::Message(TestMessageProto {
            name: "nested".to_string(),
            value: 100,
        })),
    };
}
