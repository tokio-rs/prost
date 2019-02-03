#![no_main]

use protobuf::test_messages::proto3::TestAllTypesProto3;
use tests::roundtrip;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = roundtrip::<TestAllTypesProto3>(data).unwrap_error();
});
