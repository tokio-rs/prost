include!(concat!(env!("OUT_DIR"), "/groups.rs"));

use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec::Vec;
use prost::Message;

use crate::check_message;

#[test]
fn test_group() {
    // optional group
    let msg1_bytes = &[0x0B, 0x10, 0x20, 0x0C];

    let msg1 = Test1 {
        groupa: Some(test1::GroupA { i2: Some(32) }),
    };

    let mut bytes = Vec::new();
    msg1.encode(&mut bytes).unwrap();
    assert_eq!(&bytes, msg1_bytes);

    // skip group while decoding
    let data: &[u8] = &[
        0x0B, // start group (tag=1)
        0x30, 0x01, // unused int32 (tag=6)
        0x2B, 0x30, 0xFF, 0x01, 0x2C, // unused group (tag=5)
        0x10, 0x20, // int32 (tag=2)
        0x0C, // end group (tag=1)
    ];
    assert_eq!(Test1::decode(data), Ok(msg1));

    // repeated group
    let msg2_bytes: &[u8] = &[
        0x20, 0x40, 0x2B, 0x30, 0xFF, 0x01, 0x2C, 0x2B, 0x30, 0x01, 0x2C, 0x38, 0x64,
    ];

    let msg2 = Test2 {
        i14: Some(64),
        groupb: Vec::from([
            test2::GroupB { i16: Some(255) },
            test2::GroupB { i16: Some(1) },
        ]),
        i17: Some(100),
    };

    let mut bytes = Vec::new();
    msg2.encode(&mut bytes).unwrap();
    assert_eq!(bytes.as_slice(), msg2_bytes);

    assert_eq!(Test2::decode(msg2_bytes), Ok(msg2));
}

#[test]
fn test_group_oneof() {
    let msg = OneofGroup {
        i1: Some(42),
        field: Some(oneof_group::Field::S2("foo".to_string())),
    };
    check_message(&msg);

    let msg = OneofGroup {
        i1: Some(42),
        field: Some(oneof_group::Field::G(oneof_group::G {
            i2: None,
            s1: "foo".to_string(),
            t1: None,
        })),
    };
    check_message(&msg);

    let msg = OneofGroup {
        i1: Some(42),
        field: Some(oneof_group::Field::G(oneof_group::G {
            i2: Some(99),
            s1: "foo".to_string(),
            t1: Some(Test1 {
                groupa: Some(test1::GroupA { i2: None }),
            }),
        })),
    };
    check_message(&msg);

    check_message(&OneofGroup::default());
}

#[test]
fn test_nested_group() {
    fn build_and_roundtrip(depth: usize) -> Result<(), prost::DecodeError> {
        let mut a = NestedGroup::default();
        for _ in 0..depth {
            a = NestedGroup {
                optionalgroup: Some(Box::new(nested_group::OptionalGroup {
                    nested_group: Some(a.clone()),
                })),
                requiredgroup: Box::new(nested_group::RequiredGroup {
                    nested_group: a.clone(),
                }),
                repeatedgroup: Vec::from([nested_group::RepeatedGroup {
                    nested_groups: Vec::from([a.clone()]),
                }]),
                o: Some(nested_group::O::G(Box::new(nested_group::G {
                    nested_group: Some(a.clone()),
                }))),
            };
        }

        let mut buf = Vec::new();
        a.encode(&mut buf).unwrap();
        NestedGroup::decode(buf.as_slice()).map(|_| ())
    }

    assert!(build_and_roundtrip(50).is_ok());
    assert!(build_and_roundtrip(51).is_err());
}

#[test]
fn test_deep_nesting_group() {
    fn build_and_roundtrip(depth: usize) -> Result<(), prost::DecodeError> {
        let mut a = NestedGroup2::default();
        for _ in 0..depth {
            a = NestedGroup2 {
                optionalgroup: Some(Box::new(nested_group2::OptionalGroup {
                    nested_group: Some(a),
                })),
            };
        }

        let mut buf = Vec::new();
        a.encode(&mut buf).unwrap();
        NestedGroup2::decode(buf.as_slice()).map(|_| ())
    }

    assert!(build_and_roundtrip(50).is_ok());
    assert!(build_and_roundtrip(51).is_err());
}
