#![no_main]

use libfuzzer_sys::fuzz_target;
use protobuf::test_messages::proto2::TestAllTypesProto2;
use tests::roundtrip_proto;

fuzz_target!(|data: &[u8]| {
    let _ = roundtrip_proto::<TestAllTypesProto2>(data).unwrap_error();
});
