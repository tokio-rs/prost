include!(concat!(env!("OUT_DIR"), "/type_name_prefix_suffix.rs"));

use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec::Vec;

#[test]
fn test_type_names_have_prefix_and_suffix() {
    // Create instances with both prefixed and suffixed names
    let _ = PreTestMessagePost {
        name: "test".to_string(),
        value: 42,
    };

    // Test that we can use the enum with prefix and suffix
    let test_enum = PreTestEnumPost::First;

    // Test nested types have prefix and suffix
    let _ = PreOuterMessagePost {
        inner: Some(outer_message::PreInnerMessagePost {
            data: "inner data".to_string(),
        }),
        test_enum: test_enum as i32,
    };

    // Test self-referencing types work with prefix and suffix
    let _ = PreRecursiveMessagePost {
        child: Some(Box::new(PreRecursiveMessagePost {
            child: None,
            children: Vec::new(),
        })),
        children: Vec::from([PreRecursiveMessagePost {
            child: None,
            children: Vec::new(),
        }]),
    };
}

#[test]
fn test_oneof_with_prefix_and_suffix() {
    use self::message_with_oneof::PreTestOneofPost;

    // Test that we can create oneof variants with both prefix and suffix
    let _ = PreMessageWithOneofPost {
        test_oneof: Some(PreTestOneofPost::StringValue("test".to_string())),
    };

    let _ = PreMessageWithOneofPost {
        test_oneof: Some(PreTestOneofPost::IntValue(42)),
    };

    let _ = PreMessageWithOneofPost {
        test_oneof: Some(PreTestOneofPost::Message(PreTestMessagePost {
            name: "nested".to_string(),
            value: 100,
        })),
    };
}
