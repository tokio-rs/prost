#![doc(html_root_url = "https://docs.rs/prost-build/0.1.1")]

//! `prost-build` compiles `.proto` files into Rust.
//!
//! `prost-build` is designed to be used for build-time code generation as part of a Cargo
//! build-script.
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
//!
//! ## Sourcing `protoc`
//!
//! `prost-build` depends on the Protocol Buffers compiler, `protoc`, to parse `.proto` files into
//! a representation that can be transformed into Rust. If set, `prost_build` will use the `PROTOC`
//! and `PROTOC_INCLUDE` environment variables for locating `protoc` and the protobuf built-in
//! includes. For example, on a macOS system where protobuf is installed with Homebrew, set the
//! environment to:
//!
//! ```bash
//! PROTOC=/usr/local/bin/protoc
//! PROTOC_INCLUDE=/usr/local/include
//! ```
//!
//! and in a typical Linux installation:
//!
//! ```bash
//! PROTOC=/usr/bin/protoc
//! PROTOC_INCLUDE=/usr/include
//! ```
//!
//! If `PROTOC` and `PROTOC_INCLUDE` are not found in the environment, then a pre-compiled `protoc`
//! binary will be downloaded and cached in the target directory. Pre-compiled `protoc` binaries
//! exist for Linux, macOS, and Windows systems.

extern crate bytes;
extern crate itertools;
extern crate multimap;
extern crate petgraph;
extern crate prost;
extern crate prost_types;
extern crate tempdir;

#[macro_use]
extern crate log;

mod ast;
mod code_generator;
mod ident;
mod message_graph;

use std::default;
use std::collections::HashMap;
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
use prost_types::{FileDescriptorProto, FileDescriptorSet};

pub const PROTOC: &'static str = env!("PROTOC");
pub const PROTOC_INCLUDE: &'static str = env!("PROTOC_INCLUDE");

pub use ast::{
    Comments,
    Method,
    Service,
};
use code_generator::{
    CodeGenerator,
    module,
};
use message_graph::MessageGraph;

type Module = Vec<String>;

pub trait ServiceGenerator {
    fn generate(&self, service: Service, buf: &mut String);
}

/// Configuration options for Protobuf code generation.
///
/// This configuration builder can be used to set non-default code generation options.
pub struct Config {
    service_generator: Option<Box<ServiceGenerator>>,
    btree_map: Vec<String>,
    prost_types: bool,
}

impl Config {

    /// Creates a new code generator configuration with default options.
    pub fn new() -> Config {
        Config::default()
    }

    /// Configure the code generator to generate Rust [`BTreeMap`][1] fields for Protobuf
    /// [`map`][2] type fields.
    ///
    /// # Arguments
    ///
    /// **`paths`** - paths to specific fields, messages, or packages which should use a Rust
    /// `BTreeMap` for Protobuf `map` fields. Paths are specified in terms of the Protobuf type
    /// name (not the generated Rust type name). Paths with a leading `.` are treated as fully
    /// qualified names. Paths without a leading `.` are treated as relative, and are suffix
    /// matched on the fully qualified field name. If a Protobuf map field matches any of the
    /// paths, a Rust `BTreeMap` field will be generated instead of the default [`HashMap`][3].
    ///
    /// # Examples
    ///
    /// ```
    /// # let mut config = prost_build::Config::new();
    /// // Match a specific field in a message type.
    /// config.btree_map(&[".my_messages.MyMessageType.my_map_field"]);
    ///
    /// // Match all map fields in a message type.
    /// config.btree_map(&[".my_messages.MyMessageType"]);
    ///
    /// // Match all map fields in a package.
    /// config.btree_map(&[".my_messages"]);
    ///
    /// // Match all map fields.
    /// config.btree_map(&["."]);
    ///
    /// // Match all map fields in a nested message.
    /// config.btree_map(&[".my_messages.MyMessageType.MyNestedMessageType"]);
    ///
    /// // Match all fields named 'my_map_field'.
    /// config.btree_map(&["my_map_field"]);
    ///
    /// // Match all fields named 'my_map_field' in messages named 'MyMessageType', regardless of
    /// // package or nesting.
    /// config.btree_map(&["MyMessageType.my_map_field"]);
    ///
    /// // Match all fields named 'my_map_field', and all fields in the 'foo.bar' package.
    /// config.btree_map(&["my_map_field", ".foo.bar"]);
    /// ```
    ///
    /// [1]: https://doc.rust-lang.org/std/collections/struct.BTreeMap.html
    /// [2]: https://developers.google.com/protocol-buffers/docs/proto3#maps
    /// [3]: https://doc.rust-lang.org/std/collections/struct.HashMap.html
    pub fn btree_map<I, S>(&mut self, paths: I) -> &mut Self
    where I: IntoIterator<Item = S>,
          S: AsRef<str> {
        self.btree_map = paths.into_iter().map(|s| s.as_ref().to_string()).collect();
        self
    }

    /// Configures the code generator to use the provided service generator.
    pub fn service_generator(&mut self, service_generator: Box<ServiceGenerator>) -> &mut Self {
        self.service_generator = Some(service_generator);
        self
    }

    /// Configures the code generator to not use the `prost_types` crate for Protobuf well-known
    /// types, and instead generate Protobuf well-known types from their `.proto` definitions.
    pub fn compile_well_known_types(&mut self) -> &mut Self {
        self.prost_types = false;
        self
    }

    /// Compile `.proto` files into Rust files during a Cargo build with additional code generator
    /// configuration options.
    ///
    /// This method is like the `prost_build::compile_protos` function, with the added ability to
    /// specify non-default code generation options. See that function for more information about
    /// the arguments and generated outputs.
    ///
    /// # Example `build.rs`
    ///
    /// ```norun
    /// extern crate prost_build;
    ///
    /// fn main() {
    ///     let mut prost_build = prost_build::Config::new();
    ///     prost_build.btree_map(&["."]);
    ///     prost_build.compile_protos(&["src/frontend.proto", "src/backend.proto"],
    ///                                &["src"]).unwrap();
    /// }
    /// ```
    pub fn compile_protos<P>(&self, protos: &[P], includes: &[P]) -> Result<()> where P: AsRef<Path> {
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
        // cargo, however according to [1] if we output any, those paths will
        // replace the default crate root, which we don't want. Figure out how to do
        // it in an additive way, perhaps gcc-rs has this figured out.
        // [1]: http://doc.crates.io/build-script.html#outputs-of-the-build-script

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

        let modules = self.generate(descriptor_set.file);
        for (module, content) in modules {
            let mut filename = module.join(".");
            filename.push_str(".rs");
            eprintln!("writing: {:?}", filename);
            let mut file = fs::File::create(target.join(filename))?;
            file.write_all(content.as_bytes())?;
            file.flush()?;
        }

        Ok(())
    }

    fn generate(&self, files: Vec<FileDescriptorProto>) -> HashMap<Module, String> {
        let mut modules = HashMap::new();

        let message_graph = MessageGraph::new(&files);

        for file in files {
            let module = module(&file);
            let mut buf = modules.entry(module).or_insert(String::new());
            CodeGenerator::generate(self, &message_graph, file, &mut buf);
        }
        modules
    }
}

impl default::Default for Config {
    fn default() -> Config {
        Config {
            service_generator: None,
            btree_map: Vec::new(),
            prost_types: true,
        }
    }
}

/// Compile `.proto` files into Rust files during a Cargo build.
///
/// The generated `.rs` files will be written to the Cargo `OUT_DIR` directory, suitable for use
/// with the [include!][1] macro. See the [Cargo `build.rs` code generation][2] example for more
/// info.
///
/// This function should be called in a project's `build.rs`.
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
    Config::new().compile_protos(protos, includes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_test() {
        Config::new().compile_protos(&["src/smoke_test.proto"], &["src"]).unwrap();
    }
}
