#![cfg(test)]
#![allow(clippy::float_cmp)]

use core::{f32, f64};

use protobuf::test_messages::protobuf_unittest;

#[test]
fn extreme_default_values() {
    let pb = protobuf_unittest::TestExtremeDefaultValues::default();

    assert_eq!(
        b"\0\x01\x07\x08\x0C\n\r\t\x0B\\\'\"\xFE",
        pb.escaped_bytes_fallback()
    );

    assert_eq!(0xFFFFFFFF, pb.large_uint32_fallback());
    assert_eq!(0xFFFFFFFFFFFFFFFF, pb.large_uint64_fallback());
    assert_eq!(-0x7FFFFFFF, pb.small_int32_fallback());
    assert_eq!(-0x7FFFFFFFFFFFFFFF, pb.small_int64_fallback());
    assert_eq!(-0x80000000, pb.really_small_int32_fallback());
    assert_eq!(-0x8000000000000000, pb.really_small_int64_fallback());

    assert_eq!(pb.utf8_string_fallback(), "\u{1234}");

    assert_eq!(0.0, pb.zero_float_fallback());
    assert_eq!(1.0, pb.one_float_fallback());
    assert_eq!(1.5, pb.small_float_fallback());
    assert_eq!(-1.0, pb.negative_one_float_fallback());
    assert_eq!(-1.5, pb.negative_float_fallback());
    assert_eq!(2E8, pb.large_float_fallback());
    assert_eq!(-8e-28, pb.small_negative_float_fallback());

    assert_eq!(f64::INFINITY, pb.inf_double_fallback());
    assert_eq!(f64::NEG_INFINITY, pb.neg_inf_double_fallback());
    assert_ne!(pb.nan_double_fallback(), pb.nan_double_fallback());
    assert_eq!(f32::INFINITY, pb.inf_float_fallback());
    assert_eq!(f32::NEG_INFINITY, pb.neg_inf_float_fallback());
    assert_ne!(pb.nan_float_fallback(), pb.nan_float_fallback());

    assert_eq!("? ? ?? ?? ??? ??/ ??-", pb.cpp_trigraph_fallback());

    assert_eq!("hel\x00lo", pb.string_with_zero_fallback());
    assert_eq!(b"wor\x00ld", pb.bytes_with_zero_fallback());
    assert_eq!("ab\x00c", pb.string_piece_with_zero_fallback());
    assert_eq!("12\x003", pb.cord_with_zero_fallback());
    assert_eq!("${unknown}", pb.replacement_string_fallback());
}
