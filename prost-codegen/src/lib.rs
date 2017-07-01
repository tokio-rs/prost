#![doc(html_root_url = "https://docs.rs/prost-codegen/0.1.1")]
#![recursion_limit = "128"]

#[macro_use]
extern crate prost_derive;
#[macro_use]
extern crate log;

extern crate bytes;
extern crate env_logger;
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
pub struct CodeGeneratorConfig {
    service_generator: Option<Box<ServiceGenerator>>,
}

impl CodeGeneratorConfig {

    /// Creates a new code generator with default options.
    pub fn new() -> CodeGeneratorConfig {
        CodeGeneratorConfig {
            service_generator: None,
        }
    }

    /// Configures the code generator to use the provided service generator.
    pub fn service_generator(&mut self, service_generator: Box<ServiceGenerator>) -> &mut Self {
        self.service_generator = Some(service_generator);
        self
    }
}

pub fn generate(config: &CodeGeneratorConfig, files: Vec<FileDescriptorProto>) -> HashMap<Module, String> {
    let mut modules = HashMap::new();

    let message_graph = MessageGraph::new(&files);

    for file in files {
        let module = module(&file);
        let mut buf = modules.entry(module).or_insert(String::new());
        CodeGenerator::generate(&config, &message_graph, file, &mut buf);
    }
    modules
}
