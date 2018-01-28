extern crate prost_build;
extern crate protobuf;

fn main() {
    let benchmarks = protobuf::include().join("benchmarks");
    prost_build::compile_protos(&[benchmarks.join("benchmark_messages_proto2.proto")],
                                &[benchmarks.clone()]).unwrap();
    prost_build::compile_protos(&[benchmarks.join("benchmark_messages_proto3.proto")],
                                &[benchmarks]).unwrap();

    println!("cargo:rustc-env=GOOGLE_MESSAGE1={}",
             protobuf::share().join("google_message1.dat").display());
    println!("cargo:rustc-env=GOOGLE_MESSAGE2={}",
             protobuf::share().join("google_message2.dat").display());
}
