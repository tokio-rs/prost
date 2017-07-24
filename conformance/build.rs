extern crate prost_build;
extern crate protobuf;

fn main() {
    // Emit an environment variable with the path to the conformance test runner so that it can be
    // used in the conformance tests.
    println!("cargo:rustc-env=CONFORMANCE_TEST_RUNNER={}",
             protobuf::bin().join("conformance-test-runner").display());

    let conformance = protobuf::include().join("conformance");
    prost_build::compile_protos(&[conformance.join("conformance.proto")],
                                &[conformance]).unwrap();
}
