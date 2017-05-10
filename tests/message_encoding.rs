#[macro_use]
extern crate proto_derive;

extern crate proto;
extern crate bytes;

#[macro_use]
extern crate log;
extern crate env_logger;

use std::io::Cursor;

use bytes::Buf;

use proto::Message;

// Creates a checker function for each field trait.
fn check_message<M>(msg: M) where M: Message + PartialEq {
    let expected_len = msg.encoded_len();

    let mut buf = Vec::with_capacity(18);
    msg.encode(&mut buf).unwrap();

    assert_eq!(expected_len, buf.len());

    info!("encoded message: {:?}", buf);

    let mut buf = Cursor::new(&mut buf).take(expected_len);
    let roundtrip = M::decode(&mut buf).unwrap();

    if buf.has_remaining() {
        panic!(format!("expected buffer to be empty: {}", buf.remaining()));
    }

    assert_eq!(msg, roundtrip);
}

/*
#[derive(Clone, Debug, PartialEq, Message)]
pub struct RepeatedFloats {
    #[proto(tag="11")]
    pub single_float: f32,
    #[proto(tag="41")]
    pub repeated_float: Vec<f32>,
}

#[test]
fn check_repeated_floats() {
    let _ = env_logger::init();
    check_message(RepeatedFloats { single_float: 0.0,
                                   repeated_float: vec![ 0.1,
                                                         340282300000000000000000000000000000000.0,
                                                         0.000000000000000000000000000000000000011754944 ]
    });
}
*/

/*
#[test]
fn check_scalar_types() {
    let _ = env_logger::init();
    let scalar_types = ScalarTypes {
        required_int32: 0,
        required_int64: 0,
        required_uint32: 0,
        required_uint64: 0,
        required_sint32: 0,
        required_sint64: 0,
        required_fixed32: 0,
        required_fixed64: 0,
        required_sfixed32: 0,
        required_sfixed64: 0,
        required_float: 0.0,
        required_double: 0.0,
        required_bool: false,
        required_string: String::new(),
        required_bytes: Vec::new(),

        optional_int32: None,
        optional_int64: None,
        optional_uint32: None,
        optional_uint64: None,
        optional_sint32: None,
        optional_sint64: None,
        optional_fixed32: None,
        optional_fixed64: None,
        optional_sfixed32: None,
        optional_sfixed64: None,
        optional_float: None,
        optional_double: None,
        optional_bool: None,
        optional_string: None,
        optional_bytes: None,

        repeated_int32: vec![],
        repeated_int64: vec![],
        repeated_uint32: vec![],
        repeated_uint64: vec![],
        repeated_sint32: vec![],
        repeated_sint64: vec![],
        repeated_fixed32: vec![],
        repeated_fixed64: vec![],
        repeated_sfixed32: vec![],
        repeated_sfixed64: vec![],
        repeated_float: vec![ 0.1, 340282300000000000000000000000000000000.0, 0.000000000000000000000000000000000000011754944 ],
        repeated_double: vec![],
        repeated_bool: vec![],
        repeated_string: vec![],
        repeated_bytes: vec![],

        packed_int32: vec![],
        packed_int64: vec![],
        packed_uint32: vec![],
        packed_uint64: vec![],
        packed_sint32: vec![],
        packed_sint64: vec![],
        packed_fixed32: vec![],
        packed_fixed64: vec![],
        packed_sfixed32: vec![],
        packed_sfixed64: vec![],
        packed_float: vec![],
        packed_double: vec![],
        packed_bool: vec![],
    };
    check_message(scalar_types);
}
*/

/*
/// A protobuf message which contains all scalar types.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct ScalarTypes {
    #[proto(tag="1")]
    pub required_int32: i32,
    #[proto(tag="2")]
    pub required_int64: i64,
    #[proto(tag="3")]
    pub required_uint32: u32,
    #[proto(tag="4")]
    pub required_uint64: u64,
    #[proto(signed, tag="5")]
    pub required_sint32: i32,
    #[proto(signed, tag="6")]
    pub required_sint64: i64,
    #[proto(fixed, tag="7")]
    pub required_fixed32: u32,
    #[proto(fixed, tag="8")]
    pub required_fixed64: u64,
    #[proto(fixed, tag="9")]
    pub required_sfixed32: i32,
    #[proto(fixed, tag="10")]
    pub required_sfixed64: i64,
    #[proto(tag="11")]
    pub required_float: f32,
    #[proto(tag="12")]
    pub required_double: f64,
    #[proto(tag="13")]
    pub required_bool: bool,
    #[proto(tag="14")]
    pub required_string: String,
    #[proto(tag="15")]
    pub required_bytes: Vec<u8>,

    #[proto(tag="16")]
    pub optional_int32: Option<i32>,
    #[proto(tag="17")]
    pub optional_int64: Option<i64>,
    #[proto(tag="18")]
    pub optional_uint32: Option<u32>,
    #[proto(tag="19")]
    pub optional_uint64: Option<u64>,
    #[proto(signed, tag="20")]
    pub optional_sint32: Option<i32>,
    #[proto(signed, tag="21")]
    pub optional_sint64: Option<i64>,
    #[proto(fixed, tag="22")]
    pub optional_fixed32: Option<u32>,
    #[proto(fixed, tag="23")]
    pub optional_fixed64: Option<u64>,
    #[proto(fixed, tag="24")]
    pub optional_sfixed32: Option<i32>,
    #[proto(fixed, tag="25")]
    pub optional_sfixed64: Option<i64>,
    #[proto(tag="26")]
    pub optional_float: Option<f32>,
    #[proto(tag="27")]
    pub optional_double: Option<f64>,
    #[proto(tag="28")]
    pub optional_bool: Option<bool>,
    #[proto(tag="29")]
    pub optional_string: Option<String>,
    #[proto(tag="30")]
    pub optional_bytes: Option<Vec<u8>>,

    #[proto(tag="31")]
    pub repeated_int32: Vec<i32>,
    #[proto(tag="32")]
    pub repeated_int64: Vec<i64>,
    #[proto(tag="33")]
    pub repeated_uint32: Vec<u32>,
    #[proto(tag="34")]
    pub repeated_uint64: Vec<u64>,
    #[proto(signed, tag="35")]
    pub repeated_sint32: Vec<i32>,
    #[proto(signed, tag="36")]
    pub repeated_sint64: Vec<i64>,
    #[proto(fixed, tag="37")]
    pub repeated_fixed32: Vec<u32>,
    #[proto(fixed, tag="38")]
    pub repeated_fixed64: Vec<u64>,
    #[proto(fixed, tag="39")]
    pub repeated_sfixed32: Vec<i32>,
    #[proto(fixed, tag="40")]
    pub repeated_sfixed64: Vec<i64>,
    #[proto(tag="41")]
    pub repeated_float: Vec<f32>,
    #[proto(tag="42")]
    pub repeated_double: Vec<f64>,
    #[proto(tag="43")]
    pub repeated_bool: Vec<bool>,
    #[proto(tag="44")]
    pub repeated_string: Vec<String>,
    #[proto(tag="45")]
    pub repeated_bytes: Vec<Vec<u8>>,

    // TODO: actually make these packed

    #[proto(tag="46")]
    pub packed_int32: Vec<i32>,
    #[proto(tag="47")]
    pub packed_int64: Vec<i64>,
    #[proto(tag="48")]
    pub packed_uint32: Vec<u32>,
    #[proto(tag="49")]
    pub packed_uint64: Vec<u64>,
    #[proto(signed, tag="50")]
    pub packed_sint32: Vec<i32>,
    #[proto(signed, tag="51")]
    pub packed_sint64: Vec<i64>,
    #[proto(fixed, tag="52")]
    pub packed_fixed32: Vec<u32>,
    #[proto(fixed, tag="53")]
    pub packed_fixed64: Vec<u64>,
    #[proto(fixed, tag="54")]
    pub packed_sfixed32: Vec<i32>,
    #[proto(fixed, tag="55")]
    pub packed_sfixed64: Vec<i64>,
    #[proto(tag="56")]
    pub packed_float: Vec<f32>,
    #[proto(tag="57")]
    pub packed_double: Vec<f64>,
    #[proto(tag="58")]
    pub packed_bool: Vec<bool>,
}
*/

/*
/// A protobuf message with default value.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct DefaultValues {
    #[proto(tag="1", default="42")]
    pub int32: i32,

    //#[proto(tag="2", default="Some(42)")]
    //pub optional_int32: Option<i32>,

    //#[proto(tag="3", default="\"fourty two\".to_string()")]
    //pub string: String,

}

#[test]
fn check_default_values() {
    let default = DefaultValues::default();
    assert_eq!(default.int32, 42);
    //assert_eq!(default.optional_int32, Some(42));
    //assert_eq!(&default.string, "fourty two");
    //assert_eq!(0, default.encoded_len());
}
*/


/// A protobuf message with default value.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct Basic {
    #[proto(int32, tag="1")]
    pub int32: i32,

    #[proto(bool, repeated, packed="false", tag="2")]
    pub bools: Vec<bool>,
}
