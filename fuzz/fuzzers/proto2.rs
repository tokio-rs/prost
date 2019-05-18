#![no_main]

use libfuzzer_sys::fuzz_target;
use protobuf::test_messages::proto2::TestAllTypesProto2;
use tests::roundtrip;

fuzz_target!(|data: &[u8]| {
    let _ = roundtrip::<TestAllTypesProto2>(data).unwrap_error();
});

