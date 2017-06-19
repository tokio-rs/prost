#![no_main]

#[macro_use] extern crate libfuzzer_sys;
extern crate test_all_types;

use test_all_types::test_all_types_proto3_roundtrip;

fuzz_target!(|data: &[u8]| {
    let _ = test_all_types_proto3_roundtrip(data).unwrap_error();
});
