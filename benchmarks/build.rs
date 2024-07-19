use std::path::PathBuf;

static DATASET_PROTOS: &[&str] = &[
    "google_message1/proto2/benchmark_message1_proto2.proto",
    "google_message1/proto3/benchmark_message1_proto3.proto",
    "google_message2/benchmark_message2.proto",
];

fn main() {
    let old_protobuf_benchmarks = PathBuf::from("../third_party/old_protobuf_benchmarks");

    let mut benchmark_protos = vec![old_protobuf_benchmarks.join("benchmarks.proto")];
    benchmark_protos.extend(
        DATASET_PROTOS
            .iter()
            .map(|proto| old_protobuf_benchmarks.join("datasets").join(proto)),
    );
    prost_build::compile_protos(&benchmark_protos, &[old_protobuf_benchmarks]).unwrap();
}
