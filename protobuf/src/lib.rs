#![allow(clippy::large_enum_variant, clippy::unreadable_literal)]

pub mod benchmarks {
    include!(concat!(env!("OUT_DIR"), "/benchmarks.rs"));

    pub mod dataset {
        use std::path::Path;

        pub fn google_message1_proto2() -> &'static Path {
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message1_proto2.pb"
            ))
        }

        pub fn google_message1_proto3() -> &'static Path {
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message1_proto3.pb"
            ))
        }

        pub fn google_message2() -> &'static Path {
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message2.pb"
            ))
        }

        pub fn google_message3_1() -> &'static Path {
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message3_1.pb"
            ))
        }

        pub fn google_message3_2() -> &'static Path {
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message3_2.pb"
            ))
        }

        pub fn google_message3_3() -> &'static Path {
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message3_3.pb"
            ))
        }

        pub fn google_message3_4() -> &'static Path {
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message3_4.pb"
            ))
        }

        pub fn google_message3_5() -> &'static Path {
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message3_5.pb"
            ))
        }

        pub fn google_message4() -> &'static Path {
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message4.pb"
            ))
        }
    }

    pub mod google_message3 {
        include!(concat!(env!("OUT_DIR"), "/benchmarks.google_message3.rs"));
    }
    pub mod google_message4 {
        include!(concat!(env!("OUT_DIR"), "/benchmarks.google_message4.rs"));
    }
    pub mod proto2 {
        include!(concat!(env!("OUT_DIR"), "/benchmarks.proto2.rs"));
    }
    pub mod proto3 {
        include!(concat!(env!("OUT_DIR"), "/benchmarks.proto3.rs"));
    }
}

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
