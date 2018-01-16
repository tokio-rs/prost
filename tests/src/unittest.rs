pub mod protobuf_unittest {
    include!(concat!(env!("OUT_DIR"), "/protobuf_unittest.rs"));
}

pub mod protobuf_unittest_import {
    include!(concat!(env!("OUT_DIR"), "/protobuf_unittest_import.rs"));
}

#[test]
fn extreme_default_values() {
    use std::{f32, f64};
    let pb = protobuf_unittest::TestExtremeDefaultValues::default();

    assert_eq!(b"\0\x01\x07\x08\x0C\n\r\t\x0B\\\'\"\xFE", pb.escaped_bytes());

    assert_eq!(0xFFFFFFFF, pb.large_uint32());
    assert_eq!(0xFFFFFFFFFFFFFFFF, pb.large_uint64());
    assert_eq!(-0x7FFFFFFF, pb.small_int32());
    assert_eq!(-0x7FFFFFFFFFFFFFFF, pb.small_int64());
    assert_eq!(-0x80000000, pb.really_small_int32());
    assert_eq!(-0x8000000000000000, pb.really_small_int64());

    assert_eq!(pb.utf8_string(), "\u{1234}");

    assert_eq!(0.0, pb.zero_float());
    assert_eq!(1.0, pb.one_float());
    assert_eq!(1.5, pb.small_float());
    assert_eq!(-1.0, pb.negative_one_float());
    assert_eq!(-1.5, pb.negative_float());
    assert_eq!(2E8, pb.large_float());
    assert_eq!(-8e-28, pb.small_negative_float());

    assert_eq!(f64::INFINITY, pb.inf_double());
    assert_eq!(f64::NEG_INFINITY, pb.neg_inf_double());
    assert_ne!(pb.nan_double(), pb.nan_double());
    assert_eq!(f32::INFINITY, pb.inf_float());
    assert_eq!(f32::NEG_INFINITY, pb.neg_inf_float());
    assert_ne!(pb.nan_float(), pb.nan_float());

    assert_eq!("? ? ?? ?? ??? ??/ ??-", pb.cpp_trigraph());

    assert_eq!("hel\0lo", pb.string_with_zero());
    assert_eq!(b"wor\0ld", pb.bytes_with_zero());
    assert_eq!("ab\0c", pb.string_piece_with_zero());
    assert_eq!("12\03", pb.cord_with_zero());
    assert_eq!("${unknown}", pb.replacement_string());
}
