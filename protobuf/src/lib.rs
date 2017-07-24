use std::path::{Path, PathBuf};

const PROTOBUF: &'static str = env!("PROTOBUF");

/// Returns the path to the installed Protobuf bin directory.
pub fn bin() -> PathBuf {
    Path::new(PROTOBUF).join("bin")
}

/// Returns the path to the installed Protobuf include directory.
pub fn include() -> PathBuf {
    Path::new(PROTOBUF).join("include")
}

/// Returns the path to the installed Protobuf share directory, which includes benchmarking
/// datasets.
pub fn share() -> PathBuf {
    Path::new(PROTOBUF).join("share")
}
