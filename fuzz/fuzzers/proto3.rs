#![no_main]

use libfuzzer_sys::fuzz_target;
use protobuf::test_messages::proto3::TestAllTypesProto3;
use tests::roundtrip;

fuzz_target!(|data: &[u8]| {
    let _ = roundtrip::<TestAllTypesProto3>(data).unwrap_error();
});
