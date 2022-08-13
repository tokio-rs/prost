#[rustfmt::skip]
pub mod wrappers_arc;
#[rustfmt::skip]
pub mod wrappers_box;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use super::*;

    #[test]
    fn test_wrappers_arc_default() {
        let default = wrappers_arc::MyMessage::default();
        assert_eq!(default.default_int(), 42);
        assert_eq!(default.default_float(), 1.0);
        assert_eq!(default.default_string(), "foobar");
    }

    #[test]
    fn test_wrappers_box_default() {
        let default = wrappers_box::MyMessage::default();
        assert_eq!(default.default_int(), 42);
        assert_eq!(default.default_float(), 1.0);
        assert_eq!(default.default_string(), "foobar");
    }

    #[test]
    fn test_wrappers_arc_clear() {
        use prost::Message;
        let mut default = wrappers_arc::MyMessage::default();
        default.clear();
        assert_eq!(default.default_int(), 42);
        assert_eq!(default.default_float(), 1.0);
        assert_eq!(default.default_string(), "foobar");
    }

    #[test]
    fn test_wrappers_box_clear() {
        use prost::Message;
        let mut default = wrappers_box::MyMessage::default();
        default.clear();
        assert_eq!(default.default_int(), 42);
        assert_eq!(default.default_float(), 1.0);
        assert_eq!(default.default_string(), "foobar");
    }

    #[test]
    fn test_wrappers_arc_serde() {
        use prost::Message;
        use wrappers_arc::{
            my_message::{Group, OneofField, OptionalGroup, RepeatedGroup},
            MyEnum, MyMessage, Payload,
        };

        let mut map_payload = BTreeMap::new();
        map_payload.insert(
            58,
            Payload {
                stuff: vec![58, 67, 78],
            },
        );
        map_payload.insert(
            59,
            Payload {
                stuff: vec![59, 67, 78],
            },
        );

        let msg1 = MyMessage {
            int: Arc::new(233),
            optional_int: Arc::new(Some(234)),
            repeated_int: Arc::new(vec![1, 1, 4]),
            packed_int: Arc::new(vec![5, 1, 4]),
            str: Arc::new(String::from("Do you like what you see?")),
            optional_str: Arc::new(Some(String::from("very good"))),
            repeated_str: Arc::new(vec![String::from("foo"), String::from("bar")]),
            payload: Arc::new(Payload {
                stuff: vec![55, 66, 77],
            }),
            optional_payload: Arc::new(Some(Payload {
                stuff: vec![56, 67, 78],
            })),
            repeated_payload: Arc::new(vec![
                Payload {
                    stuff: vec![56, 67, 78],
                },
                Payload {
                    stuff: vec![57, 68, 79],
                },
            ]),
            map_payload: Arc::new(map_payload),
            group: Arc::new(Group { i2: Some(5885) }),
            optional_group: Arc::new(Some(OptionalGroup { i2: Some(85858959) })),
            repeated_group: Arc::new(vec![
                RepeatedGroup { i2: Some(7) },
                RepeatedGroup { i2: Some(8) },
                RepeatedGroup { i2: Some(9) },
            ]),
            oneof_field: Arc::new(Some(OneofField::B(vec![6, 6, 6]))),
            my_enum: Arc::new(MyEnum::Baz as i32),
            optional_my_enum: Arc::new(Some(MyEnum::Foo as i32)),
            repeated_my_enum: Arc::new(vec![MyEnum::Foo as i32, MyEnum::Bar as i32]),
            packed_my_enum: Arc::new(vec![MyEnum::Bar as i32, MyEnum::Foo as i32]),
            default_int: Arc::new(Some(889)),
            default_float: Arc::new(Some(888.)),
            default_string: Arc::new(Some(String::from("that's simple"))),
        };
        let bytes = msg1.encode_to_vec();
        let msg2 = MyMessage::decode(&*bytes).unwrap();
        assert_eq!(msg1, msg2);
    }

    #[test]
    fn test_wrappers_box_serde() {
        use prost::Message;
        use wrappers_box::{
            my_message::{Group, OneofField, OptionalGroup, RepeatedGroup},
            MyEnum, MyMessage, Payload,
        };

        let mut map_payload = BTreeMap::new();
        map_payload.insert(
            58,
            Payload {
                stuff: vec![58, 67, 78],
            },
        );
        map_payload.insert(
            59,
            Payload {
                stuff: vec![59, 67, 78],
            },
        );

        let msg1 = MyMessage {
            int: Box::new(233),
            optional_int: Box::new(Some(234)),
            repeated_int: Box::new(vec![1, 1, 4]),
            packed_int: Box::new(vec![5, 1, 4]),
            str: Box::new(String::from("Do you like what you see?")),
            optional_str: Box::new(Some(String::from("very good"))),
            repeated_str: Box::new(vec![String::from("foo"), String::from("bar")]),
            payload: Box::new(Payload {
                stuff: vec![55, 66, 77],
            }),
            optional_payload: Box::new(Some(Payload {
                stuff: vec![56, 67, 78],
            })),
            repeated_payload: Box::new(vec![
                Payload {
                    stuff: vec![56, 67, 78],
                },
                Payload {
                    stuff: vec![57, 68, 79],
                },
            ]),
            map_payload: Box::new(map_payload),
            group: Box::new(Group { i2: Some(5885) }),
            optional_group: Box::new(Some(OptionalGroup { i2: Some(85858959) })),
            repeated_group: Box::new(vec![
                RepeatedGroup { i2: Some(7) },
                RepeatedGroup { i2: Some(8) },
                RepeatedGroup { i2: Some(9) },
            ]),
            oneof_field: Box::new(Some(OneofField::B(vec![6, 6, 6]))),
            my_enum: Box::new(MyEnum::Baz as i32),
            optional_my_enum: Box::new(Some(MyEnum::Foo as i32)),
            repeated_my_enum: Box::new(vec![MyEnum::Foo as i32, MyEnum::Bar as i32]),
            packed_my_enum: Box::new(vec![MyEnum::Bar as i32, MyEnum::Foo as i32]),
            default_int: Box::new(Some(889)),
            default_float: Box::new(Some(888.)),
            default_string: Box::new(Some(String::from("that's simple"))),
        };
        let bytes = msg1.encode_to_vec();
        let msg2 = MyMessage::decode(&*bytes).unwrap();
        assert_eq!(msg1, msg2);
    }
}
