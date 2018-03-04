#![no_main]

#[macro_use] extern crate libfuzzer_sys;
extern crate tests;

use tests::protobuf_test_messages::proto3::TestAllTypesProto3;
use tests::roundtrip;

fuzz_target!(|data: &[u8]| {
    let _ = roundtrip::<TestAllTypesProto3>(data).unwrap_error();
});
