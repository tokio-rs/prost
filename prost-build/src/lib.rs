//! `prost-build` is a small library which makes it easy to add build-time code generation of
//! `.proto` files to a Cargo project.
//!
//! ## Example
//!
//! Let's create a small crate, `snazzy`, that defines a collection of
//! snazzy new items in a protobuf file.
//!
//! ```bash
//! $ cargo new snazzy && cd snazzy
//! ```
//!
//! First, add `prost-build`, `prost` and its public dependencies to `Cargo.toml`
//! (see [crates.io](https://crates.io/crates/prost) for the current versions):
//!
//! ```toml
//! [dependencies]
//! bytes = <bytes-version>
//! prost = <prost-version>
//! prost-derive = <prost-version>
//!
//! [build-dependencies]
//! prost-build = <prost-version>
//! ```
//!
//! Next, add `src/items.proto` to the project:
//!
//! ```proto
//! syntax = "proto3";
//!
//! package snazzy.items;
//!
//! // A snazzy new shirt!
//! message Shirt {
//! enum Size {
//!     SMALL = 0;
//!     MEDIUM = 1;
//!     LARGE = 2;
//! }
//!
//! string color = 1;
//! Size size = 2;
//! }
//! ```
//!
//! To generate Rust code from `items.proto`, we use `prost-build` in the crate's
//! `build.rs` build-script:
//!
//! ```rust,no_run
//! extern crate prost_build;
//!
//! fn main() {
//!     prost_build::compile_protos(&["src/items.proto"],
//!                                 &["src/"]).unwrap();
//! }
//! ```
//!
//! And finally, in `lib.rs`, include the generated code:
//!
//! ```rust,ignore
//! extern crate prost;
//! #[macro_use]
//! extern crate prost_derive;
//!
//! // Include the `items` module, which is generated from items.proto.
//! pub mod items {
//!     include!(concat!(env!("OUT_DIR"), "/items.rs"));
//! }
//!
//! pub fn create_large_shirt(color: String) -> items::Shirt {
//!     let mut shirt = items::Shirt::default();
//!     shirt.color = color;
//!     shirt.set_size(items::shirt::Size::Large);
//!     shirt
//! }
//! ```
//!
//! That's it! Run `cargo doc` to see documentation for the generated code. The full
//! example project can be found on [GitHub](https://github.com/danburkert/snazzy).

#![doc(html_root_url = "https://docs.rs/prost-build/0.1.1")]

extern crate bytes;
extern crate prost;
extern crate prost_codegen;
extern crate tempdir;

use std::env;
use std::fs;
use std::io::{
    Error,
    ErrorKind,
    Read,
    Result,
    Write,
};
use std::path::{
    Path,
    PathBuf,
};
use std::process::Command;

use prost::Message;
use prost_codegen::google::protobuf::FileDescriptorSet;

pub const PROTOC: &'static str = env!("PROTOC");
pub const PROTOC_INCLUDE: &'static str = env!("PROTOC_INCLUDE");

/// Compile `.proto` files into Rust files during a Cargo build.
///
/// The generated `.rs` files will be written to the Cargo `OUT_DIR` directory, suitable for use
/// with the [include!][1] macro. See the [Cargo `build.rs` code generation][2] example for more
/// info.
///
/// This function should be called in a project's `build.rs`.
///
/// # Sourcing `protoc`
///
/// `prost` uses the Protocol Buffers compiler, `protoc`, to parse `.proto` files into a
/// representation that can be transformed into Rust. If present, `compile_protos` will use the
/// `PROTOC` and `PROTOC_INCLUDE` environment variables for locating `protoc` and the protobuf
/// built-in includes. For example, on a macOS system where protobuf is installed with Homebrew,
/// set the environment to:
///
/// ```bash
/// PROTOC=/usr/local/bin/protoc
/// PROTOC_INCLUDE=/usr/local/include
/// ```
///
/// and in a typical Linux installation:
///
/// ```bash
/// PROTOC=/usr/bin/protoc
/// PROTOC_INCLUDE=/usr/include
/// ```
///
/// If `PROTOC` and `PROTOC_INCLUDE` are not found in the environment, then a pre-compiled `protoc`
/// binary will be downloaded and cached in the target directory. Pre-compiled `protoc` binaries
/// exist for Linux, macOS, and Windows systems.
///
/// # Arguments
///
/// **`protos`** - Paths to `.proto` files to compile. Any transitively [imported][3] `.proto`
/// files will automatically be included.
///
/// **`includes`** - Paths to directories in which to search for imports. Directories will be
/// searched in order. The `.proto` files passed in **`protos`** must be found
/// in one of the provided include directories.
///
/// # Errors
///
/// This function can fail for a number of reasons:
///
///   - Failure to locate or download `protoc`.
///   - Failure to parse the `.proto`s.
///   - Failure to locate an imported `.proto`.
///
/// It's expected that this function call be `unwrap`ed in a `build.rs`; there is typically no
/// reason to gracefully recover from errors during a build.
///
/// # Example `build.rs`
///
/// ```norun
/// extern crate prost_build;
///
/// fn main() {
///     prost_build::compile_protos(&["src/frontend.proto", "src/backend.proto"],
///                                 &["src"]).unwrap();
/// }
/// ```
///
/// [1]: https://doc.rust-lang.org/std/macro.include.html
/// [2]: http://doc.crates.io/build-script.html#case-study-code-generation
/// [3]: https://developers.google.com/protocol-buffers/docs/proto3#importing-definitions
pub fn compile_protos<P>(protos: &[P], includes: &[P]) -> Result<()> where P: AsRef<Path> {
    compile_protos_with_config(&prost_codegen::Config::new(), protos, includes)
}

/// Compile `.proto` files into Rust files during a Cargo build with additional code generator
/// configuration options.
///
/// See `prost_codegen::Config` for the available options. Other than providing additional control
/// over the generated code, this function works identically to `compile_protos`.
///
/// # Example `build.rs`
///
/// ```norun
/// extern crate prost_build;
/// extern crate prost_codegen;
///
/// fn main() {
///     prost_build::compile_protos_with_config(prost_codegen::Config::new().btree_map(&["."]),
///                                             &["src/frontend.proto", "src/backend.proto"],
///                                             &["src"]).unwrap();
/// }
/// ```
pub fn compile_protos_with_config<P>(config: &prost_codegen::Config,
                                     protos: &[P],
                                     includes: &[P])
                                     -> Result<()> where P: AsRef<Path> {

    let target = match env::var("OUT_DIR") {
        Ok(val) => PathBuf::from(val),
        Err(env::VarError::NotPresent) => {
            return Err(Error::new(ErrorKind::Other,
                                  "OUT_DIR environment variable is not set"));
        },
        Err(env::VarError::NotUnicode(..)) => {
            return Err(Error::new(ErrorKind::InvalidData,
                                  "OUT_DIR environment variable is not valid UTF-8"));
        },
    };

    // TODO: We should probably emit 'rerun-if-changed=PATH' directives for
    // cargo, however according to
    // http://doc.crates.io/build-script.html#outputs-of-the-build-script if we
    // output any, those paths will replace the default crate root, which we
    // don't want. Figure out how to do it in an additive way, perhaps gcc-rs
    // has this figured out.


    let tmp = tempdir::TempDir::new("prost-build")?;
    let descriptor_set = tmp.path().join("prost-descriptor-set");

    let mut cmd = Command::new(PROTOC);
    cmd.arg("--include_imports")
       .arg("--include_source_info")
       .arg("-o").arg(&descriptor_set);

    for include in includes {
        cmd.arg("-I").arg(include.as_ref());
    }

    // Set the protoc include after the user includes in case the user wants to
    // override one of the built-in .protos.
    cmd.arg("-I").arg(PROTOC_INCLUDE);

    for proto in protos {
        cmd.arg(proto.as_ref());
    }

    let output = cmd.output()?;
    if !output.status.success() {
        return Err(Error::new(ErrorKind::Other,
                              format!("protoc failed: {}",
                                      String::from_utf8_lossy(&output.stderr))));
    }

    let mut buf = Vec::new();
    fs::File::open(descriptor_set)?.read_to_end(&mut buf)?;
    let descriptor_set = FileDescriptorSet::decode(&buf)?;

    let modules = prost_codegen::generate(config, descriptor_set.file);
    for (module, content) in modules {
        let mut filename = match module.last() {
            Some(filename) => PathBuf::from(filename),
            None => return Err(Error::new(ErrorKind::InvalidInput, ".proto must have a package")),
        };
        filename.set_extension("rs");
        let mut file = fs::File::create(target.join(filename))?;
        file.write_all(content.as_bytes())?;
        file.flush()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_test() {
        compile_protos(&["src/smoke_test.proto"], &["src"]).unwrap();
    }
}
