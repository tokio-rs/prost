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
};
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

mod descriptor;
mod plugin;

fn main() {
    env_logger::init().unwrap();
    let mut bytes = Vec::new();
    io::stdin().read_to_end(&mut bytes).unwrap();

    assert_ne!(bytes.len(), 0);

    let mut request = plugin::CodeGeneratorRequest::default();
    Message::merge_from(&mut request, bytes.len(), &mut Cursor::new(&mut bytes)).unwrap();

    let mut response = plugin::CodeGeneratorResponse::default();

    trace!("{:#?}", request);

    let mut files: HashMap<String, String> = HashMap::new();

    for file in request.proto_file {
        let mut name = file.package.split('.').join("/");
        name.push_str(".rs");
        let content = CodeGenerator::generate(file);
        match files.entry(name) {
            hash_map::Entry::Occupied(mut entry) => entry.get_mut().push_str(&content),
            hash_map::Entry::Vacant(mut entry) => {
                entry.insert(content);
            },
        }
    }

    for (name, content) in files {
        let mut file = plugin::code_generator_response::File::default();
        file.name = name;
        file.content = content;
        response.file.push(file);
    }

    let out = io::stdout();
    response.write_to(&mut out.lock()).unwrap();
}

struct CodeGenerator {
    source_info: descriptor::SourceCodeInfo,
    depth: u8,
    path: Vec<i32>,
    buf: String,
}

impl CodeGenerator {
    fn generate(file: descriptor::FileDescriptorProto) -> String {

        let mut source_info = file.source_code_info.expect("no source code info in request");
        source_info.location.retain(|location| {
            let len = location.path.len();
            len > 0 && len % 2 == 0
        });
        source_info.location.sort_by_key(|location| location.path.clone());


        let mut code_gen = CodeGenerator {
            source_info: source_info,
            depth: 0,
            path: Vec::new(),
            buf: String::new(),
        };

        debug!("file: {:?}, package: {:?}", file.name, file.package);

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

        code_gen.buf
    }

    fn append_message(&mut self, message: descriptor::DescriptorProto) {
        debug!("  message: {:?}", message.name);

        self.append_doc();
        self.push_indent();
        self.buf.push_str("#[derive(Debug, Message)]\n");
        self.push_indent();
        self.buf.push_str("pub struct ");
        self.buf.push_str(&message.name);
        self.buf.push_str(" {\n");

        self.depth += 1;
        self.path.push(2);
        for (idx, field) in message.fields.into_iter().enumerate() {
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

    fn append_field(&mut self, field: descriptor::FieldDescriptorProto) {
        use descriptor::field_descriptor_proto::Type::*;
        use descriptor::field_descriptor_proto::Label::*;

        let repeated = field.label == LABEL_REPEATED;
        let signed = field.field_type == TYPE_SINT32 ||
                     field.field_type == TYPE_SINT64;
        let fixed = field.field_type == TYPE_FIXED32 ||
                    field.field_type == TYPE_FIXED64 ||
                    field.field_type == TYPE_SFIXED32 ||
                    field.field_type == TYPE_SFIXED64;
        let message = field.field_type == TYPE_MESSAGE;

        let ty = match field.field_type {
            TYPE_FLOAT => Cow::Borrowed("f32"),
            TYPE_DOUBLE => Cow::Borrowed("f64"),
            TYPE_UINT32 | TYPE_FIXED32 => Cow::Borrowed("u32"),
            TYPE_UINT64 | TYPE_FIXED64 => Cow::Borrowed("u64"),
            TYPE_INT32 | TYPE_SFIXED32 | TYPE_SINT32 => Cow::Borrowed("i32"),
            TYPE_INT64 | TYPE_SFIXED64 | TYPE_SINT64 => Cow::Borrowed("i64"),
            TYPE_BOOL => Cow::Borrowed("bool"),
            TYPE_STRING => Cow::Borrowed("String"),
            TYPE_BYTES => Cow::Borrowed("Vec<u8>"),
            TYPE_GROUP | TYPE_MESSAGE | TYPE_ENUM => Cow::Owned(rust_ident(&field.type_name)),
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
                self.buf.push_str("//!");
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

    fn append_enum(&mut self, desc: descriptor::EnumDescriptorProto) {
        debug!("  enum: {:?}", desc.name);

        self.append_doc();
        self.push_indent();
        self.buf.push_str("#[derive(Clone, Copy, Debug, Enumeration)]\n");
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

    fn append_enum_value(&mut self, value: descriptor::EnumValueDescriptorProto) {
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
        self.buf.push_str("mod ");
        self.buf.push_str(module);
        self.buf.push_str(" {\n");
        self.depth += 1;
    }

    fn pop_mod(&mut self) {
        self.depth -= 1;
        self.push_indent();
        self.buf.push_str("}\n");
    }
}

/// Converts a Protobuf identifier (e.g. `.Foo.Bar`) to Rust (e.g. `foo::Bar`).
fn rust_ident(pb_ident: &str) -> String {
    let mut ident = rust_module(pb_ident).map(|mut module| {
        module.push_str("::");
        module
    }).unwrap_or_default();
    ident.push_str(rust_type(pb_ident));
    ident
}

/// Returns the Rust module from a Protobuf identifier.
fn rust_module(pb_ident: &str) -> Option<String> {
    let ridx = pb_ident.rfind('.').expect(&format!("malformed Protobuf identifier: {}", pb_ident));

    if ridx == 0 {
        None
    } else {
        Some(pb_ident[1..ridx].split('.').map(camel_to_snake).join("::"))
    }
}

fn rust_type(pb_ident: &str) -> &str {
    let idx = pb_ident.rfind('.').expect(&format!("malformed Protobuf identifier: {}", pb_ident));
    &pb_ident[idx + 1..]
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
