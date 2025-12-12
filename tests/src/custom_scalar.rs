include!(concat!(env!("OUT_DIR"), "/custom_scalar.rs"));

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use prost::Message;

#[test]
fn test_custom_scalar() {
    let msg = Msg {
        a: MyString("a".into()),
        b: vec![MyString("b".into())],
        c: Some(MyString("c".into())),
        my_enum: Some(msg::MyEnum::D(MyString("e".into()))),
        e: [(MyString("f".into()), MyString("f".into()))]
            .iter()
            .cloned()
            .collect(),
        f: [("f".into(), MyString("f".into()))]
            .iter()
            .cloned()
            .collect(),
        g: [(MyString("f".into()), "f".into())]
            .iter()
            .cloned()
            .collect(),
        h: MyVec(vec![1, 2]),
    };

    let data = msg.encode_to_vec();
    let decoded_msg = Msg::decode(data.as_slice()).unwrap();

    assert_eq!(msg, decoded_msg);
}

#[derive(Clone, Default, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct MyString(pub String);

pub struct MyStringInterface;

impl prost::CustomScalarInterface for MyStringInterface {
    type Type = MyString;
    type RefType<'x> = &'x str;

    fn encoded_len(tag: u32, value: &Self::Type) -> usize {
        ::prost::encoding::string::encoded_len(tag, &value.0)
    }

    fn encode(tag: u32, value: &Self::Type, buf: &mut impl prost::bytes::BufMut) {
        ::prost::encoding::string::encode(tag, &value.0, buf);
    }

    fn merge(
        wire_type: prost::encoding::WireType,
        value: &mut Self::Type,
        buf: &mut impl prost::bytes::Buf,
        ctx: prost::encoding::DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        ::prost::encoding::string::merge(wire_type, &mut value.0, buf, ctx)
    }

    fn is_default(value: &Self::Type) -> bool {
        value.0.is_empty()
    }

    fn get<'x>(value: &'x Option<Self::Type>) -> Self::RefType<'x> {
        match value {
            Some(value) => value.0.as_str(),
            None => "",
        }
    }
}

#[derive(Clone, Default, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct MyVec(pub Vec<u8>);

struct MyVecInterface;

impl prost::CustomScalarInterface for MyVecInterface {
    type Type = MyVec;
    type RefType<'x> = &'x [u8];

    fn encoded_len(tag: u32, value: &Self::Type) -> usize {
        ::prost::encoding::bytes::encoded_len(tag, &value.0)
    }

    fn encode(tag: u32, value: &Self::Type, buf: &mut impl prost::bytes::BufMut) {
        ::prost::encoding::bytes::encode(tag, &value.0, buf);
    }

    fn merge(
        wire_type: prost::encoding::WireType,
        value: &mut Self::Type,
        buf: &mut impl prost::bytes::Buf,
        ctx: prost::encoding::DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        ::prost::encoding::bytes::merge(wire_type, &mut value.0, buf, ctx)
    }

    fn is_default(value: &Self::Type) -> bool {
        value.0.is_empty()
    }

    fn get<'x>(value: &'x Option<Self::Type>) -> Self::RefType<'x> {
        match value {
            Some(value) => value.0.as_slice(),
            None => &[],
        }
    }
}
