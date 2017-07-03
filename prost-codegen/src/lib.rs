#![doc(html_root_url = "https://docs.rs/prost-codegen/0.1.1")]
#![recursion_limit = "128"]

#[macro_use]
extern crate prost_derive;
#[macro_use]
extern crate log;

extern crate bytes;
extern crate env_logger;
extern crate heck;
extern crate itertools;
extern crate multimap;
extern crate petgraph;
extern crate prost;

mod ast;
mod code_generator;
mod ident;
mod message_graph;
pub mod google;

use std::collections::HashMap;

pub use ast::{
    Comments,
    Method,
    Service,
};
use code_generator::{
    CodeGenerator,
    module,
};
use google::protobuf::FileDescriptorProto;
use message_graph::MessageGraph;

pub type Module = Vec<String>;

pub trait ServiceGenerator {
    fn generate(&self, service: Service, buf: &mut String);
}

/// Configuration options for Protobuf code generation.
#[derive(Default)]
pub struct CodeGeneratorConfig {
    service_generator: Option<Box<ServiceGenerator>>,
    btree_map: Vec<String>,
}

impl CodeGeneratorConfig {

    /// Creates a new code generator with default options.
    pub fn new() -> CodeGeneratorConfig {
        CodeGeneratorConfig::default()
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
    /// # let mut config = prost_codegen::CodeGeneratorConfig::new();
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
}

pub fn generate(config: &CodeGeneratorConfig,
                files: Vec<FileDescriptorProto>)
                -> HashMap<Module, String> {
    let mut modules = HashMap::new();

    let message_graph = MessageGraph::new(&files);

    for file in files {
        let module = module(&file);
        let mut buf = modules.entry(module).or_insert(String::new());
        CodeGenerator::generate(&config, &message_graph, file, &mut buf);
    }
    modules
}
