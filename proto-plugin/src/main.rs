#[macro_use]
extern crate proto_derive;
#[macro_use]
extern crate log;

extern crate env_logger;
extern crate itertools;
extern crate proto;

use std::borrow::Cow;
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
    let mut file = plugin::code_generator_response::File::default();

    let mut s = String::new();
    for file in request.proto_file {
        append_file(file, &mut s);

    }

    file.content = s;
    file.name = "fub.rs".to_string();
    response.file.push(file);

    let out = io::stdout();
    response.write_to(&mut out.lock()).unwrap();
}

fn append_file(file: descriptor::FileDescriptorProto, s: &mut String) {
    debug!("file: {:?}, package: {:?}", file.name, file.package);
    if file.package.is_empty() {
        for message in file.message_type {
            append_message(message, 0, s);
        }
    } else {
        push_mod(&file.package, 0, s);
        for message in file.message_type {
            append_message(message, 1, s);
        }
        pop_mod(0, s);
    }
}

fn append_message(message: descriptor::DescriptorProto, depth: u8, s: &mut String) {
    if !message.nested_type.is_empty() {
        push_mod(&camel_to_snake(&message.name), depth, s);

        for inner in message.nested_type {
            append_message(inner, depth + 1, s);
        }
        pop_mod(depth, s);
    }

    debug!("  message: {:?}", message.name);
    push_indent(depth, s);
    s.push_str("struct ");
    s.push_str(&message.name);
    s.push_str(" {\n");

    for field in message.fields {
        append_field(field, depth + 1, s);
    }

    push_indent(depth, s);
    s.push_str("}\n");
}

fn append_field(field: descriptor::FieldDescriptorProto, depth: u8, s: &mut String) {
    use descriptor::field_descriptor_proto::Type::*;
    use descriptor::field_descriptor_proto::Label::*;

    let repeated = field.label == LABEL_REPEATED;
    let mut signed = field.field_type == TYPE_SINT32 ||
                     field.field_type == TYPE_SINT64;
    let mut fixed = field.field_type == TYPE_FIXED32 ||
                    field.field_type == TYPE_FIXED64 ||
                    field.field_type == TYPE_SFIXED32 ||
                    field.field_type == TYPE_SFIXED64;

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

    push_indent(depth, s);
    s.push_str("#[proto(tag=");
    s.push_str(&field.number.to_string());
    if signed {
        s.push_str(", signed");
    } else if fixed {
        s.push_str(", fixed");
    }
    s.push_str(")]\n");
    push_indent(depth, s);
    s.push_str("pub ");
    s.push_str(&field.name);
    s.push_str(": ");
    if repeated { s.push_str("Vec<"); }
    s.push_str(&ty);
    if repeated { s.push_str(">"); }
    s.push_str(",\n");
}

fn push_indent(depth: u8, s: &mut String) {
    for _ in 0..depth {
        s.push_str("    ");
    }
}

fn push_mod(module: &str, depth: u8, s: &mut String) {
    push_indent(depth, s);
    s.push_str("mod ");
    s.push_str(module);
    s.push_str(" {\n");
}

fn pop_mod(depth: u8, s: &mut String) {
    push_indent(depth, s);
    s.push_str("}\n");
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
}
