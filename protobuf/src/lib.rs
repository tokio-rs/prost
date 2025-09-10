#![allow(clippy::large_enum_variant)]

pub mod conformance {
    use std::path::Path;

    pub fn test_runner() -> &'static Path {
        Path::new(concat!(env!("PROTOBUF"), "/bin/conformance-test-runner"))
    }

    include!(concat!(env!("OUT_DIR"), "/conformance.rs"));
}

pub mod test_messages {
    pub mod proto2 {
        include!(concat!(
            env!("OUT_DIR"),
            "/protobuf_test_messages.proto2.rs"
        ));
    }
    pub mod proto3 {
        include!(concat!(
            env!("OUT_DIR"),
            "/protobuf_test_messages.proto3.rs"
        ));
    }
    pub mod protobuf_unittest {
        include!(concat!(env!("OUT_DIR"), "/protobuf_unittest.rs"));
    }
    pub mod protobuf_unittest_import {
        include!(concat!(env!("OUT_DIR"), "/protobuf_unittest_import.rs"));
    }
}
