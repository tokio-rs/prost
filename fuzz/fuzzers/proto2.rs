#![no_main]

use protobuf::test_messages::proto2::TestAllTypesProto2;
use tests::roundtrip;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let _ = roundtrip::<TestAllTypesProto2>(data).unwrap_error();
});

