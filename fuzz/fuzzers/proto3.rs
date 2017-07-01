#![no_main]

#[macro_use] extern crate libfuzzer_sys;
extern crate test_all_types;

use test_all_types::protobuf_test_messages::proto3::TestAllTypes;
use test_all_types::roundtrip;

fuzz_target!(|data: &[u8]| {
    let _ = roundtrip::<TestAllTypes>(data).unwrap_error();
});
