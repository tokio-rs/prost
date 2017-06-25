extern crate bytes;
extern crate env_logger;
extern crate itertools;
extern crate prost;
extern crate prost_codegen;

use std::collections::HashMap;
use std::io::{
    Cursor,
    Read,
    Write,
    self,
};
use std::path::PathBuf;

use bytes::Buf;

use prost::Message;
use prost_codegen::google::protobuf::compiler::{
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

    let modules = prost_codegen::generate(request.proto_file, None);

    // For each module, build up a list of its child modules.
    let mut children: HashMap<prost_codegen::Module, Vec<String>> = HashMap::new();
    for module in modules.keys() {
        for i in 0..module.len() {
            children.entry(module[..i].to_owned())
                    .or_insert_with(Default::default)
                    .push(module[i].clone());
        }
    }

    // Create each module.
    for (module, buf) in modules {
        let mut children = children.remove(&module).unwrap_or_default();

        let mut path = module.into_iter().collect::<PathBuf>();
        children.sort();
        children.dedup();

        if !children.is_empty() || path.iter().count() == 0 {
            path.push("mod");
        }
        path.set_extension("rs");

        let mut content = String::new();

        for child in children {
            content.push_str("pub mod ");
            content.push_str(&child);
            content.push_str(";\n");
        }

        content.push_str(&buf);

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
