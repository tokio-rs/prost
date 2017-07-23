#![no_main]

#[macro_use] extern crate libfuzzer_sys;
extern crate tests;

use tests::protobuf_test_messages::proto3::TestAllTypes;
use tests::roundtrip;

fuzz_target!(|data: &[u8]| {
    let _ = roundtrip::<TestAllTypes>(data).unwrap_error();
});
