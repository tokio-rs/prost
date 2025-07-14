// This test ensures we can compile using re-exported dependencies as configured in
// `build.rs`. Note that there's no direct dependency of `::prost` or `::prost-types` in
// `Cargo.toml`.
include!(concat!(env!("OUT_DIR"), "/prost_path.rs"));
