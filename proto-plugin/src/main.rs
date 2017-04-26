#![recursion_limit = "128"]

#[macro_use]
extern crate proto_derive;
#[macro_use]
extern crate log;

extern crate bytes;
extern crate env_logger;
extern crate itertools;
extern crate multimap;
extern crate proto;

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::io::{
    Cursor,
    Read,
    Write,
    self,
};

use bytes::Buf;

use itertools::{Either, Itertools};
use multimap::MultiMap;
use proto::Message;

mod google;
use google::protobuf::{
    DescriptorProto,
    EnumDescriptorProto,
    EnumValueDescriptorProto,
    FieldDescriptorProto,
    FileDescriptorProto,
    OneofDescriptorProto,
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
            CodeGenerator::generate(file, &mut content);
        }

        response.file.push(code_generator_response::File {
            name: path.to_string_lossy().into_owned(),
            content: content,
            ..Default::default()
        });
    }

    let mut out = Vec::new();
    response.encode(&mut out).unwrap();
    io::stdout().write_all(&out).unwrap();
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

        // Split the nested message types into a vector of normal nested message types, and a map
        // of the map field entry types. The path index of the nested message types is preserved so
        // that comments can be retrieved.
        let message_name = message.name;
        let nested_types = message.nested_type;
        let (nested_types, map_types): (Vec<(DescriptorProto, usize)>, HashMap<String, (FieldDescriptorProto, FieldDescriptorProto)>) =
            nested_types.into_iter().enumerate().partition_map(|(idx, nested_type)| {
                if nested_type.options.as_ref().map(|options| options.map_entry).unwrap_or(false) {
                    let key = nested_type.field[0].clone();
                    let value = nested_type.field[1].clone();
                    assert_eq!(&key.name, "key");
                    assert_eq!(&value.name, "value");

                    let name = format!(".{}.{}.{}", self.package, &message_name, nested_type.name);
                    Either::Right((name, (key, value)))
                } else {
                    Either::Left((nested_type, idx))
                }
        });

        // Split the fields into a vector of the normal fields, and oneof field. Path indexes are
        // preserved so that comments can be retrieved.
        let (fields, mut oneof_fields): (Vec<(FieldDescriptorProto, usize)>, MultiMap<i32, (FieldDescriptorProto, usize)>) =
            message.field.into_iter().enumerate().partition_map(|(idx, field)| {
                if field.oneof_index >= 0 {
                    Either::Right((field.oneof_index, (field, idx)))
                } else {
                    Either::Left((field, idx))
                }
            });

        assert_eq!(oneof_fields.len(), message.oneof_decl.len());

        self.append_doc();
        self.push_indent();
        self.buf.push_str("#[derive(Clone, Debug, PartialEq, Message)]\n");
        self.push_indent();
        self.buf.push_str("pub struct ");
        self.buf.push_str(&message_name);
        self.buf.push_str(" {\n");

        self.depth += 1;
        self.path.push(2);
        for (field, idx) in fields.into_iter() {
            self.path.push(idx as i32);
            match map_types.get(&field.type_name) {
                Some(&(ref key, ref value)) => self.append_map_field(field, key, value),
                None => self.append_field(field),
            }
            self.path.pop();
        }
        self.path.pop();

        self.path.push(8);
        for (idx, oneof) in message.oneof_decl.iter().enumerate() {
            let idx = idx as i32;
            self.path.push(idx);
            self.append_oneof_field(&message_name, oneof, &oneof_fields.get_vec(&idx).unwrap());
            self.path.pop();
        }
        self.path.pop();

        self.depth -= 1;
        self.push_indent();
        self.buf.push_str("}\n");

        if !message.enum_type.is_empty() || !nested_types.is_empty() || !oneof_fields.is_empty() {
            self.push_mod(&message_name);
            self.path.push(3);
            for (nested_type, idx) in nested_types.into_iter() {
                self.path.push(idx as i32);
                self.append_message(nested_type);
                self.path.pop();
            }
            self.path.pop();

            self.path.push(4);
            for (idx, nested_enum) in message.enum_type.into_iter().enumerate() {
                self.path.push(idx as i32);
                self.append_enum(nested_enum);
                self.path.pop();
            }
            self.path.pop();

            for (idx, oneof) in message.oneof_decl.into_iter().enumerate() {
                let idx = idx as i32;
                self.append_oneof(oneof, idx, oneof_fields.remove(&idx).unwrap());
            }

            self.pop_mod();
        }
    }

    fn append_field(&mut self, field: FieldDescriptorProto) {
        use field_descriptor_proto::Type::*;
        use field_descriptor_proto::Label::*;

        let repeated = field.label == LabelRepeated;
        let message = field.field_type == TypeMessage;
        let type_modifier = field_type_modifier(field.field_type);
        let ty = self.resolve_type(&field);

        debug!("    field: {:?}, type: {:?}", field.name, ty);

        self.append_doc();
        self.push_indent();
        self.buf.push_str("#[proto(tag=\"");
        self.buf.push_str(&field.number.to_string());
        self.buf.push_str("\"");
        if let Some(modifier) = type_modifier {
            self.buf.push_str(", ");
            self.buf.push_str(modifier);
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

    fn append_map_field(&mut self,
                        field: FieldDescriptorProto,
                        key: &FieldDescriptorProto,
                        value: &FieldDescriptorProto) {
        let key_type_modifier = field_type_modifier(key.field_type);
        let value_type_modifier = field_type_modifier(value.field_type);
        let key_ty = self.resolve_type(key);
        let value_ty = self.resolve_type(value);

        debug!("    map field: {:?}, key type: {:?}, value type: {:?}", field.name, key_ty, value_ty);

        self.append_doc();
        self.push_indent();
        self.buf.push_str("#[proto(tag=\"");
        self.buf.push_str(&field.number.to_string());
        self.buf.push_str("\"");
        if let Some(modifier) = key_type_modifier {
            self.buf.push_str(", ");
            self.buf.push_str(modifier);
            self.buf.push_str("_key");
        }
        if let Some(modifier) = value_type_modifier {
            self.buf.push_str(", ");
            self.buf.push_str(modifier);
            self.buf.push_str("_value");
        }
        self.buf.push_str(")]\n");
        self.push_indent();
        self.buf.push_str("pub ");
        self.buf.push_str(&field.name);
        self.buf.push_str(": ::std::collections::HashMap<");
        self.buf.push_str(&key_ty);
        self.buf.push_str(", ");
        self.buf.push_str(&value_ty);
        self.buf.push_str(">,\n");
    }

    fn append_oneof_field(&mut self,
                          message_name: &str,
                          oneof: &OneofDescriptorProto,
                          fields: &[(FieldDescriptorProto, usize)]) {
        self.append_doc();
        self.push_indent();
        self.buf.push_str(&format!("#[proto({})]\n",
                                   fields.iter()
                                         .format_with(", ", |&(ref field, _), f| {
                                             f(&format_args!("tag=\"{}\"", field.number))
                                         })));
        self.push_indent();
        self.buf.push_str(&format!("{}: Option<{}::{}>,\n",
                                   oneof.name,
                                   camel_to_snake(message_name),
                                   snake_to_upper_camel(&oneof.name)));
    }

    fn append_oneof(&mut self,
                    oneof: OneofDescriptorProto,
                    idx: i32,
                    fields: Vec<(FieldDescriptorProto, usize)>) {
        self.path.push(8);
        self.path.push(idx);
        self.append_doc();
        self.path.pop();
        self.path.pop();

        self.push_indent();
        self.buf.push_str("#[derive(Clone, Debug, PartialEq, Oneof)]\n");
        self.push_indent();
        self.buf.push_str("pub enum ");
        self.buf.push_str(&snake_to_upper_camel(&oneof.name));
        self.buf.push_str(" {\n");

        self.path.push(2);
        self.depth += 1;
        for (field, idx) in fields {
            self.path.push(idx as i32);
            self.append_doc();
            self.path.pop();

            let type_modifier = field_type_modifier(field.field_type);

            self.push_indent();
            self.buf.push_str("#[proto(tag=\"");
            self.buf.push_str(&field.number.to_string());
            self.buf.push_str("\"");
            if let Some(modifier) = type_modifier {
                self.buf.push_str(", ");
                self.buf.push_str(modifier);
            }
            self.buf.push_str(")]\n");

            self.push_indent();
            let name = snake_to_upper_camel(&field.name);
            self.buf.push_str(&name);
            self.buf.push_str("(");
            let ty = self.resolve_type(&field);
            self.buf.push_str(&ty);
            self.buf.push_str("),\n");
        }
        self.depth -= 1;
        self.path.pop();

        self.push_indent();
        self.buf.push_str("}\n");
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
        self.buf.push_str(&camel_to_snake(module));
        self.buf.push_str(" {\n");

        self.package.push_str(".");
        self.package.push_str(module);

        self.depth += 1;
    }

    fn pop_mod(&mut self) {
        self.depth -= 1;

        let idx = self.package.rfind('.').unwrap();
        self.package.truncate(idx);

        self.push_indent();
        self.buf.push_str("}\n");
    }

    fn resolve_type<'b>(&self, field: &'b FieldDescriptorProto) -> Cow<'b, str> {
        use field_descriptor_proto::Type::*;
        match field.field_type {
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
        }
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

fn field_type_modifier(field_type: field_descriptor_proto::Type) -> Option<&'static str> {
    use field_descriptor_proto::Type::*;
    match field_type {
        TypeSint32 | TypeSint64 => Some("signed"),
        TypeFixed32 | TypeFixed64 | TypeSfixed32 | TypeSfixed64 => Some("fixed"),
        _ => None,
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
