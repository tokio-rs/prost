extern crate prost_build;
extern crate protobuf;

fn main() {
    let benchmarks = protobuf::include().join("benchmarks");
    let benchmarks = benchmarks.as_path();
    let datasets = benchmarks.join("datasets");
    let datasets = datasets.as_path();
    prost_build::compile_protos(
        &[
            benchmarks.join("benchmarks.proto"),
            datasets.join("google_message1").join("proto2").join("benchmark_message1_proto2.proto"),
            datasets.join("google_message1").join("proto2").join("benchmark_message1_proto2.proto"),
            datasets.join("google_message1").join("proto3").join("benchmark_message1_proto3.proto"),
            datasets.join("google_message2").join("benchmark_message2.proto"),
            datasets.join("google_message3").join("benchmark_message3.proto"),
            datasets.join("google_message4").join("benchmark_message4.proto"),
        ], &[benchmarks.to_path_buf()]).unwrap();

    println!("cargo:rustc-env=DATASET_GOOGLE_MESSAGE1_PROTO2={}",
             datasets.join("google_message1").join("proto2").join("dataset.google_message1_proto2.pb").display());
    println!("cargo:rustc-env=DATASET_GOOGLE_MESSAGE1_PROTO3={}",
             datasets.join("google_message1").join("proto3").join("dataset.google_message1_proto3.pb").display());
    println!("cargo:rustc-env=DATASET_GOOGLE_MESSAGE2={}",
             datasets.join("google_message2").join("dataset.google_message2.pb").display());
}
