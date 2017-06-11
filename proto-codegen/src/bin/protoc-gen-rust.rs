#![recursion_limit = "128"]

#[macro_use]
extern crate log;

extern crate bytes;
extern crate env_logger;
extern crate itertools;
extern crate proto;

extern crate proto_codegen;

use std::collections::HashMap;
use std::io::{
    Cursor,
    Read,
    Write,
    self,
};
use std::path::PathBuf;

use bytes::Buf;

use proto::Message;

use proto_codegen::google::protobuf::FileDescriptorProto;
use proto_codegen::google::protobuf::compiler::{
    code_generator_response,
    CodeGeneratorRequest,
    CodeGeneratorResponse,
};

fn main() {
    env_logger::init().unwrap();
    let mut bytes = Vec::new();
    io::stdin().read_to_end(&mut bytes).unwrap();

    let len = bytes.len();
    assert_ne!(len, 0);

    let request = CodeGeneratorRequest::decode(&mut Buf::take(Cursor::new(&mut bytes), len)).unwrap();
    let mut response = CodeGeneratorResponse::default();

    trace!("{:#?}", request);

    #[derive(Default)]
    struct Module {
        children: Vec<String>,
        files: Vec<FileDescriptorProto>,
    }

    // Map from module path to module.
    let mut modules: HashMap<Vec<String>, Module> = HashMap::new();

    // Step 1: For each .proto file, add it to the module map,
    // as well as an entry for each parent package.
    for file in request.proto_file {
        let path = proto_codegen::module(&file);

        for i in 0..path.len() {
            modules.entry(path[..i].to_owned())
                    .or_insert_with(Default::default)
                    .children
                    .push(path[i].clone());
        }

        modules.entry(path)
               .or_insert_with(Default::default)
               .files.push(file);
    }

    // Step 2: Create each module.
    for (path, mut module) in modules {
        let mut path = path.into_iter().collect::<PathBuf>();
        module.children.sort();
        module.children.dedup();

        if !module.children.is_empty() || path.iter().count() == 0 {
            path.push("mod");
        }
        path.set_extension("rs");

        let mut content = String::new();

        for child in module.children {
            content.push_str("pub mod ");
            content.push_str(&child);
            content.push_str(";\n");
        }

        for file in module.files {
            proto_codegen::generate(file, &mut content);
        }

        response.file.push(code_generator_response::File {
            name: Some(path.to_string_lossy().into_owned()),
            content: Some(content),
            ..Default::default()
        });
    }

    let mut out = Vec::new();
    response.encode(&mut out).unwrap();
    io::stdout().write_all(&out).unwrap();
}
