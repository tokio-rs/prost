pub mod protobuf_unittest {
    include!(concat!(env!("OUT_DIR"), "/protobuf_unittest.rs"));
}

pub mod protobuf_unittest_import {
    include!(concat!(env!("OUT_DIR"), "/protobuf_unittest_import.rs"));
}

// GeneratedMessageTest.FloatingPointDefaults
#[test]
fn floating_point_defaults() {
    use std::{f32, f64};
    let extreme_default = protobuf_unittest::TestExtremeDefaultValues::default();
    assert_eq!(0.0f32, extreme_default.zero_float());
    assert_eq!(1.0f32, extreme_default.one_float());
    assert_eq!(1.5f32, extreme_default.small_float());
    assert_eq!(-1.0f32, extreme_default.negative_one_float());
    assert_eq!(-1.5f32, extreme_default.negative_float());
    assert_eq!(2.0e8f32, extreme_default.large_float());
    assert_eq!(-8e-28f32, extreme_default.small_negative_float());
    assert_eq!(f64::INFINITY, extreme_default.inf_double());
    assert_eq!(f64::NEG_INFINITY, extreme_default.neg_inf_double());
    assert_ne!(extreme_default.nan_double(), extreme_default.nan_double());
    assert_eq!(f32::INFINITY, extreme_default.inf_float());
    assert_eq!(f32::NEG_INFINITY, extreme_default.neg_inf_float());
    assert_ne!(extreme_default.nan_float(), extreme_default.nan_float());
}

// GeneratedMessageTest.ExtremeSmallIntegerDefault
#[test]
fn extreme_small_integer_default() {
    use std::{i32, i64};
    let extreme_default = protobuf_unittest::TestExtremeDefaultValues::default();
    assert_eq!(i32::MIN, extreme_default.really_small_int32());
    assert_eq!(i64::MIN, extreme_default.really_small_int64());
}

// GeneratedMessageTest.StringDefaults
#[test]
fn string_defaults() {
  let message = protobuf_unittest::TestExtremeDefaultValues::default();
  // Check if '\000' can be used in default string value.
  assert_eq!("hel\0lo", message.string_with_zero());
  assert_eq!(b"wor\0ld", message.bytes_with_zero());
}
