#[macro_use]
extern crate prost_derive;

pub mod benchmarks {
    include!(concat!(env!("OUT_DIR"), "/benchmarks.rs"));

    use std::path::Path;

    pub fn datasets() -> Vec<&'static Path> {
        vec![
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message1_proto2.pb"
            )),
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message1_proto3.pb"
            )),
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message2.pb"
            )),
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message3_1.pb"
            )),
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message3_2.pb"
            )),
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message3_3.pb"
            )),
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message3_4.pb"
            )),
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message3_5.pb"
            )),
            Path::new(concat!(
                env!("PROTOBUF"),
                "/share/dataset.google_message4.pb"
            )),
        ]
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
