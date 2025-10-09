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
    pub mod proto2_unittest {
        include!(concat!(env!("OUT_DIR"), "/proto2_unittest.rs"));
    }
    pub mod proto2_unittest_import {
        include!(concat!(env!("OUT_DIR"), "/proto2_unittest_import.rs"));
    }

    pub mod editions {
        pub mod proto2 {
            include!(concat!(
                env!("OUT_DIR"),
                "/protobuf_test_messages.editions.proto2.rs"
            ));
        }
        pub mod proto3 {
            include!(concat!(
                env!("OUT_DIR"),
                "/protobuf_test_messages.editions.proto3.rs"
            ));
        }
        pub mod edition2023 {
            include!(concat!(
                env!("OUT_DIR"),
                "/protobuf_test_messages.editions.rs"
            ));
        }
        // Re-export for convenience
        pub use edition2023::TestAllTypesEdition2023;
    }

    pub use proto2_unittest as protobuf_unittest;
    pub use proto2_unittest_import as protobuf_unittest_import;
}
