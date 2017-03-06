#[macro_use]
extern crate proto_derive;
#[macro_use]
extern crate log;

extern crate env_logger;
extern crate itertools;
extern crate proto;

use std::ascii::AsciiExt;
use std::borrow::Cow;
use std::collections::{
    hash_map,
    HashMap,
    HashSet,
};
use std::path::PathBuf;
use std::io::{
    Cursor,
    Read,
    Result,
    Write,
    self,
};
use std::mem;

use itertools::Itertools;
use proto::Message;

mod google;
use google::protobuf::{
    DescriptorProto,
    EnumDescriptorProto,
    EnumValueDescriptorProto,
    FieldDescriptorProto,
    FileDescriptorProto,
    SourceCodeInfo,
    field_descriptor_proto,

};
use google::protobuf::compiler::{
    CodeGeneratorRequest,
    CodeGeneratorResponse,
    code_generator_response,
};

fn main() {
    env_logger::init().unwrap();
    let mut bytes = Vec::new();
    io::stdin().read_to_end(&mut bytes).unwrap();

    assert_ne!(bytes.len(), 0);

    let mut request = CodeGeneratorRequest::default();
    Message::merge_from(&mut request, bytes.len(), &mut Cursor::new(&mut bytes)).unwrap();

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
        let path = file.package
                       .split('.')
                       .filter(|s| !s.is_empty())
                       .map(camel_to_snake)
                       .collect::<Vec<_>>();

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

        if !module.children.is_empty() {
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
            CodeGenerator::generate(file, &mut content);
        }

        response.file.push(code_generator_response::File {
            name: path.to_string_lossy().into_owned(),
            content: content,
            ..Default::default()
        });
    }

    let out = io::stdout();
    response.write_to(&mut out.lock()).unwrap();
}

struct CodeGenerator<'a> {
    package: String,
    source_info: SourceCodeInfo,
    depth: u8,
    path: Vec<i32>,
    buf: &'a mut String,
}

impl <'a> CodeGenerator<'a> {
    fn generate(file: FileDescriptorProto, buf: &mut String) {

        let mut source_info = file.source_code_info.expect("no source code info in request");
        source_info.location.retain(|location| {
            let len = location.path.len();
            len > 0 && len % 2 == 0
        });
        source_info.location.sort_by_key(|location| location.path.clone());

        let mut code_gen = CodeGenerator {
            package: file.package,
            source_info: source_info,
            depth: 0,
            path: Vec::new(),
            buf: buf,
        };

        debug!("file: {:?}, package: {:?}", file.name, code_gen.package);

        code_gen.path.push(4);
        for (idx, message) in file.message_type.into_iter().enumerate() {
            code_gen.path.push(idx as i32);
            code_gen.append_message(message);
            code_gen.path.pop();
        }
        code_gen.path.pop();

        code_gen.path.push(5);
        for (idx, desc) in file.enum_type.into_iter().enumerate() {
            code_gen.path.push(idx as i32);
            code_gen.append_enum(desc);
            code_gen.path.pop();
        }
        code_gen.path.pop();
    }

    fn append_message(&mut self, message: DescriptorProto) {
        debug!("  message: {:?}", message.name);

        self.append_doc();
        self.push_indent();
        self.buf.push_str("#[derive(Debug, PartialEq, Message)]\n");
        self.push_indent();
        self.buf.push_str("pub struct ");
        self.buf.push_str(&message.name);
        self.buf.push_str(" {\n");

        self.depth += 1;
        self.path.push(2);
        for (idx, field) in message.field.into_iter().enumerate() {
            self.path.push(idx as i32);
            self.append_field(field);
            self.path.pop();
        }
        self.path.pop();
        self.depth -= 1;
        self.push_indent();
        self.buf.push_str("}\n");

        if !message.nested_type.is_empty() || !message.enum_type.is_empty() {
            self.push_mod(&camel_to_snake(&message.name));
            self.path.push(3);
            for (idx, inner) in message.nested_type.into_iter().enumerate() {
                self.path.push(idx as i32);
                self.append_message(inner);
                self.path.pop();
            }
            self.path.pop();

            self.path.push(4);
            for (idx, inner) in message.enum_type.into_iter().enumerate() {
                self.path.push(idx as i32);
                self.append_enum(inner);
                self.path.pop();
            }
            self.path.pop();

            self.pop_mod();
        }
    }

    fn append_field(&mut self, field: FieldDescriptorProto) {
        use field_descriptor_proto::Type::*;
        use field_descriptor_proto::Label::*;

        let repeated = field.label == LabelRepeated;
        let signed = field.field_type == TypeSint32 ||
                     field.field_type == TypeSint64;
        let fixed = field.field_type == TypeFixed32 ||
                    field.field_type == TypeFixed64 ||
                    field.field_type == TypeSfixed32 ||
                    field.field_type == TypeSfixed64;
        let message = field.field_type == TypeMessage;

        let ty = match field.field_type {
            TypeFloat => Cow::Borrowed("f32"),
            TypeDouble => Cow::Borrowed("f64"),
            TypeUint32 | TypeFixed32 => Cow::Borrowed("u32"),
            TypeUint64 | TypeFixed64 => Cow::Borrowed("u64"),
            TypeInt32 | TypeSfixed32 | TypeSint32 => Cow::Borrowed("i32"),
            TypeInt64 | TypeSfixed64 | TypeSint64 => Cow::Borrowed("i64"),
            TypeBool => Cow::Borrowed("bool"),
            TypeString => Cow::Borrowed("String"),
            TypeBytes => Cow::Borrowed("Vec<u8>"),
            TypeGroup | TypeMessage | TypeEnum => Cow::Owned(self.resolve_ident(&field.type_name)),
        };
        debug!("    field: {:?}, type: {:?}", field.name, ty);

        self.append_doc();
        self.push_indent();
        self.buf.push_str("#[proto(tag=\"");
        self.buf.push_str(&field.number.to_string());
        self.buf.push_str("\"");
        if signed {
            self.buf.push_str(", signed");
        } else if fixed {
            self.buf.push_str(", fixed");
        }
        self.buf.push_str(")]\n");
        self.push_indent();
        self.buf.push_str("pub ");
        self.buf.push_str(&field.name);
        self.buf.push_str(": ");
        if repeated { self.buf.push_str("Vec<"); }
        else if message { self.buf.push_str("Option<"); }
        self.buf.push_str(&ty);
        if repeated || message { self.buf.push_str(">"); }
        self.buf.push_str(",\n");
    }

    fn append_doc(&mut self) {
        let idx = self.source_info
                      .location
                      .binary_search_by_key(&&self.path[..], |location| &location.path[..])
                      .unwrap();
        let location = &self.source_info.location[idx];

        for comment in &location.leading_detached_comments {
            for line in comment.lines() {
                for _ in 0..self.depth {
                    self.buf.push_str("    ");
                }
                self.buf.push_str("//");
                self.buf.push_str(line);
                self.buf.push_str("\n");
            }
            self.buf.push_str("\n");
        }

        for line in location.leading_comments.lines() {
            for _ in 0..self.depth {
                self.buf.push_str("    ");
            }
            self.buf.push_str("///");
            self.buf.push_str(line);
            self.buf.push_str("\n");
        }
        for line in location.trailing_comments.lines() {
            for _ in 0..self.depth {
                self.buf.push_str("    ");
            }
            self.buf.push_str("///");
            self.buf.push_str(line);
            self.buf.push_str("\n");
        }
    }

    fn append_enum(&mut self, desc: EnumDescriptorProto) {
        debug!("  enum: {:?}", desc.name);

        self.append_doc();
        self.push_indent();
        self.buf.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq, Enumeration)]\n");
        self.push_indent();
        self.buf.push_str("pub enum ");
        self.buf.push_str(&desc.name);
        self.buf.push_str(" {\n");

        self.depth += 1;
        self.path.push(2);
        for (idx, value) in desc.value.into_iter().enumerate() {
            self.path.push(idx as i32);
            self.append_enum_value(value);
            self.path.pop();
        }
        self.path.pop();
        self.depth -= 1;

        self.push_indent();
        self.buf.push_str("}\n");

    }

    fn append_enum_value(&mut self, value: EnumValueDescriptorProto) {
        self.append_doc();
        self.push_indent();
        self.buf.push_str(&snake_to_upper_camel(&value.name));
        self.buf.push_str(" = ");
        self.buf.push_str(&value.number.to_string());
        self.buf.push_str(",\n");
    }

    fn push_indent(&mut self) {
        for _ in 0..self.depth {
            self.buf.push_str("    ");
        }
    }

    fn push_mod(&mut self, module: &str) {
        self.push_indent();
        self.buf.push_str("pub mod ");
        self.buf.push_str(module);
        self.buf.push_str(" {\n");
        self.depth += 1;
    }

    fn pop_mod(&mut self) {
        self.depth -= 1;
        self.push_indent();
        self.buf.push_str("}\n");
    }

    fn resolve_ident(&self, pb_ident: &str) -> String {
        // protoc should always give fully qualified identifiers.
        assert_eq!(".", &pb_ident[..1]);

        let mut local_path = self.package.split('.').peekable();

        let mut ident_path = pb_ident[1..].split('.');
        let ident_type = ident_path.next_back().unwrap();
        let mut ident_path = ident_path.peekable();

        // Skip path elements in common.
        while local_path.peek().is_some() &&
              local_path.peek() == ident_path.peek() {
            local_path.next();
            ident_path.next();
        }

        local_path.map(|_| "super".to_string())
                  .chain(ident_path.map(camel_to_snake))
                  .chain(Some(ident_type.to_string()).into_iter())
                  .join("::")
    }
}

fn camel_to_snake(camel: &str) -> String {
    // protoc does not allow non-ascii identifiers.
    let len = camel.as_bytes().iter().skip(1).filter(|&&c| is_uppercase(c)).count() + camel.len();
    let mut snake = Vec::with_capacity(len);

    let mut skip = 0;
    for (i, &c) in camel.as_bytes().iter().enumerate() {
        if is_uppercase(c) {
            if i != skip {
                snake.push('_' as u8);
                skip = i + 1;
            } else {
                skip += 1;
            }
            snake.push(to_lowercase(c));
        } else {
            snake.push(c);
        }
    }

    String::from_utf8(snake).expect(&format!("non-utf8 identifier: {}", camel))
}

fn snake_to_upper_camel(snake: &str) -> String {
    let mut s = String::with_capacity(snake.len());

    if snake.is_empty() {
        return s;
    }

    for fragment in snake.split('_') {
        if fragment.is_empty() {
            s.push('_');
        } else {
            let (first, rest) = fragment.split_at(1);
            s.push_str(&first.to_uppercase());
            s.push_str(&rest.to_lowercase());
        }
    }
    s
}

#[inline]
fn is_uppercase(c: u8) -> bool {
    c >= 'A' as u8 && c <= 'Z' as u8
}

#[inline]
fn to_lowercase(c: u8) -> u8 {
    debug_assert!(is_uppercase(c));
    c + 32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camel_to_snake() {
        assert_eq!("foo_bar", &camel_to_snake("FooBar"));
        assert_eq!("foo_bar_baz", &camel_to_snake("FooBarBAZ"));
        assert_eq!("foo_bar_baz", &camel_to_snake("FooBArBAZ"));
        assert_eq!("foo_bar_bazle_e", &camel_to_snake("FooBArBAZleE"));
    }

    #[test]
    fn test_snake_to_upper_camel() {
        assert_eq!("", &snake_to_upper_camel(""));
        assert_eq!("F", &snake_to_upper_camel("F"));
        assert_eq!("Foo", &snake_to_upper_camel("FOO"));
        assert_eq!("FooBar", &snake_to_upper_camel("FOO_BAR"));
        assert_eq!("_FooBar", &snake_to_upper_camel("_FOO_BAR"));
        assert_eq!("FooBar_", &snake_to_upper_camel("FOO_BAR_"));
        assert_eq!("_FooBar_", &snake_to_upper_camel("_FOO_BAR_"));
        assert_eq!("Fuzzbuster", &snake_to_upper_camel("fuzzBuster"));
    }
}
