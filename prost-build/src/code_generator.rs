use std::ascii;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use itertools::{Either, Itertools};
use multimap::MultiMap;
use prost_types::{
    DescriptorProto,
    EnumDescriptorProto,
    EnumValueDescriptorProto,
    FieldDescriptorProto,
    FileDescriptorProto,
    OneofDescriptorProto,
    ServiceDescriptorProto,
    SourceCodeInfo,
};
use prost_types::field_descriptor_proto::{Label, Type};
use prost_types::source_code_info::Location;

use ast::{
    Comments,
    Method,
    Service,
};
use ident::{
    to_snake,
    match_ident,
    to_upper_camel,
};
use message_graph::MessageGraph;
use Config;
use Module;

pub fn module(file: &FileDescriptorProto) -> Module {
    file.package()
        .split('.')
        .filter(|s| !s.is_empty())
        .map(to_snake)
        .collect()
}

#[derive(PartialEq)]
enum Syntax {
    Proto2,
    Proto3,
}

pub struct CodeGenerator<'a> {
    config: &'a mut Config,
    package: String,
    source_info: SourceCodeInfo,
    syntax: Syntax,
    message_graph: &'a MessageGraph,
    depth: u8,
    path: Vec<i32>,
    buf: &'a mut String,
}

impl <'a> CodeGenerator<'a> {
    pub fn generate(config: &mut Config,
                    message_graph: &MessageGraph,
                    file: FileDescriptorProto,
                    buf: &mut String) {

        let mut source_info = file.source_code_info.expect("no source code info in request");
        source_info.location.retain(|location| {
            let len = location.path.len();
            len > 0 && len % 2 == 0
        });
        source_info.location.sort_by_key(|location| location.path.clone());

        let syntax = match file.syntax.as_ref().map(String::as_str) {
            None | Some("proto2") => Syntax::Proto2,
            Some("proto3") => Syntax::Proto3,
            Some(s) => panic!("unknown syntax: {}", s),
        };

        let mut code_gen = CodeGenerator {
            config: config,
            package: file.package.unwrap(),
            source_info: source_info,
            syntax: syntax,
            message_graph: message_graph,
            depth: 0,
            path: Vec::new(),
            buf: buf,
        };

        debug!("file: {:?}, package: {:?}", file.name.as_ref().unwrap(), code_gen.package);

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

        if code_gen.config.service_generator.is_some() {
            code_gen.path.push(6);
            for (idx, service) in file.service.into_iter().enumerate() {
                code_gen.path.push(idx as i32);
                code_gen.push_service(service);
                code_gen.path.pop();
            }

            let buf = code_gen.buf;
            code_gen.config.service_generator.as_mut().map(|service_generator| {
                service_generator.finalize(buf);
            });

            code_gen.path.pop();
        }
    }

    fn append_message(&mut self, message: DescriptorProto) {
        debug!("  message: {:?}", message.name());

        let message_name = message.name().to_string();
        let fq_message_name = format!(".{}.{}", self.package, message.name());

        // Skip Protobuf well-known types.
        if self.well_known_type(&fq_message_name).is_some() { return; }

        // Split the nested message types into a vector of normal nested message types, and a map
        // of the map field entry types. The path index of the nested message types is preserved so
        // that comments can be retrieved.
        type NestedTypes = Vec<(DescriptorProto, usize)>;
        type MapTypes = HashMap<String, (FieldDescriptorProto, FieldDescriptorProto)>;
        let (nested_types, map_types): (NestedTypes, MapTypes) =
            message.nested_type.into_iter().enumerate().partition_map(|(idx, nested_type)| {
                if nested_type.options.as_ref().and_then(|options| options.map_entry).unwrap_or(false) {
                    let key = nested_type.field[0].clone();
                    let value = nested_type.field[1].clone();
                    assert_eq!("key", key.name());
                    assert_eq!("value", value.name());

                    let name = format!("{}.{}", &fq_message_name, nested_type.name());
                    Either::Right((name, (key, value)))
                } else {
                    Either::Left((nested_type, idx))
                }
        });

        // Split the fields into a vector of the normal fields, and oneof fields.
        // Path indexes are preserved so that comments can be retrieved.
        type Fields = Vec<(FieldDescriptorProto, usize)>;
        type OneofFields = MultiMap<i32, (FieldDescriptorProto, usize)>;
        let (fields, mut oneof_fields): (Fields, OneofFields) =
            message.field.into_iter().enumerate().partition_map(|(idx, field)| {
                if let Some(oneof_index) = field.oneof_index {
                    Either::Right((oneof_index, (field, idx)))
                } else {
                    Either::Left((field, idx))
                }
            });

        assert_eq!(oneof_fields.len(), message.oneof_decl.len());

        self.append_doc();
        self.push_indent();
        self.buf.push_str("#[derive(Clone, PartialEq, Message)]\n");
        self.append_type_attributes(&fq_message_name);
        self.push_indent();
        self.buf.push_str("pub struct ");
        self.buf.push_str(&to_upper_camel(&message_name));
        self.buf.push_str(" {\n");

        self.depth += 1;
        self.path.push(2);
        for (field, idx) in fields {
            self.path.push(idx as i32);
            match field.type_name.as_ref().and_then(|type_name| map_types.get(type_name)) {
                Some(&(ref key, ref value)) => self.append_map_field(&fq_message_name, field, key, value),
                None => self.append_field(&fq_message_name, field),
            }
            self.path.pop();
        }
        self.path.pop();

        self.path.push(8);
        for (idx, oneof) in message.oneof_decl.iter().enumerate() {
            let idx = idx as i32;
            self.path.push(idx);
            self.append_oneof_field(&message_name,
                                    &fq_message_name,
                                    oneof,
                                    oneof_fields.get_vec(&idx).unwrap());
            self.path.pop();
        }
        self.path.pop();

        self.depth -= 1;
        self.push_indent();
        self.buf.push_str("}\n");

        if !message.enum_type.is_empty() || !nested_types.is_empty() || !oneof_fields.is_empty() {
            self.push_mod(&message_name);
            self.path.push(3);
            for (nested_type, idx) in nested_types {
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
                self.append_oneof(&fq_message_name, oneof, idx, oneof_fields.remove(&idx).unwrap());
            }

            self.pop_mod();
        }
    }

    fn append_type_attributes(&mut self, msg_name: &str) {
        assert_eq!(b'.', msg_name.as_bytes()[0]);
        // TODO: this clone is dirty, but expedious.
        for (matcher, attribute) in self.config.type_attributes.clone() {
            if match_ident(&matcher, msg_name, None) {
                self.push_indent();
                self.buf.push_str(&attribute);
                self.buf.push('\n');
            }
        }
    }

    fn append_field_attributes(&mut self, msg_name: &str, field_name: &str) {
        assert_eq!(b'.', msg_name.as_bytes()[0]);
        // TODO: this clone is dirty, but expedious.
        for (matcher, attribute) in self.config.field_attributes.clone() {
            if match_ident(&matcher, msg_name, Some(field_name)) {
                self.push_indent();
                self.buf.push_str(&attribute);
                self.buf.push('\n');
            }
        }
    }

    fn append_field(&mut self, msg_name: &str, field: FieldDescriptorProto) {
        // TODO(danburkert/prost#19): support groups.
        let type_ = field.type_();
        if type_ == Type::Group { return; }

        let repeated = field.label == Some(Label::Repeated as i32);
        let optional = self.optional(&field);
        let ty = self.resolve_type(&field);

        let boxed = !repeated
                 && type_ == Type::Message
                 && self.message_graph.is_nested(field.type_name(), msg_name);

        debug!("    field: {:?}, type: {:?}, boxed: {}", field.name(), ty, boxed);

        self.append_doc();
        self.push_indent();
        self.buf.push_str("#[prost(");
        let type_tag = self.field_type_tag(&field);
        self.buf.push_str(&type_tag);

        match field.label() {
            Label::Optional => if optional {
                self.buf.push_str(", optional");
            },
            Label::Required => self.buf.push_str(", required"),
            Label::Repeated => {
                self.buf.push_str(", repeated");
                if can_pack(&field) && !field.options.as_ref().map_or(self.syntax == Syntax::Proto3,
                                                                      |options| options.packed()) {
                    self.buf.push_str(", packed=\"false\"");
                }
            },
        }

        if boxed { self.buf.push_str(", boxed"); }
        self.buf.push_str(", tag=\"");
        self.buf.push_str(&field.number().to_string());

        if let Some(ref default) = field.default_value {
            self.buf.push_str("\", default=\"");
            if type_ == Type::Bytes {
                self.buf.push_str("b\\\"");
                for b in unescape_c_escape_string(default) {
                    self.buf.extend(ascii::escape_default(b).flat_map(|c| (c as char).escape_default()));
                }
                self.buf.push_str("\\\"");
            } else if type_ == Type::Enum {
                self.buf.push_str(&to_upper_camel(default));
            } else {
                // TODO: this is only correct if the Protobuf escaping matches Rust escaping. To be
                // safer, we should unescape the Protobuf string and re-escape it with the Rust
                // escaping mechanisms.
                self.buf.push_str(default);
            }
        }

        self.buf.push_str("\")]\n");
        self.append_field_attributes(msg_name, field.name());
        self.push_indent();
        self.buf.push_str("pub ");
        self.buf.push_str(&to_snake(field.name()));
        self.buf.push_str(": ");
        if repeated { self.buf.push_str("::std::vec::Vec<"); }
        else if optional { self.buf.push_str("::std::option::Option<"); }
        if boxed { self.buf.push_str("::std::boxed::Box<"); }
        self.buf.push_str(&ty);
        if boxed { self.buf.push_str(">"); }
        if repeated || optional { self.buf.push_str(">"); }
        self.buf.push_str(",\n");
    }

    fn append_map_field(&mut self,
                        msg_name: &str,
                        field: FieldDescriptorProto,
                        key: &FieldDescriptorProto,
                        value: &FieldDescriptorProto) {
        let key_ty = self.resolve_type(key);
        let value_ty = self.resolve_type(value);

        debug!("    map field: {:?}, key type: {:?}, value type: {:?}",
               field.name(), key_ty, value_ty);

        self.append_doc();
        self.push_indent();

        let btree_map = self.config
                            .btree_map
                            .iter()
                            .any(|matcher| match_ident(matcher, msg_name, Some(field.name())));
        let (annotation_ty, rust_ty) = if btree_map {
            ("btree_map", "BTreeMap")
        } else {
            ("map", "HashMap")
        };

        let key_tag = self.field_type_tag(key);
        let value_tag = self.map_value_type_tag(value);
        self.buf.push_str(&format!("#[prost({}=\"{}, {}\", tag=\"{}\")]\n",
                                   annotation_ty,
                                   key_tag,
                                   value_tag,
                                   field.number()));
        self.append_field_attributes(msg_name, field.name());
        self.push_indent();
        self.buf.push_str(&format!("pub {}: ::std::collections::{}<{}, {}>,\n",
                                   to_snake(field.name()), rust_ty, key_ty, value_ty));
    }

    fn append_oneof_field(&mut self,
                          message_name: &str,
                          fq_message_name: &str,
                          oneof: &OneofDescriptorProto,
                          fields: &[(FieldDescriptorProto, usize)]) {
        let name = format!("{}::{}",
                           to_snake(message_name),
                           to_upper_camel(oneof.name()));
        self.append_doc();
        self.push_indent();
        self.buf.push_str(&format!("#[prost(oneof=\"{}\", tags=\"{}\")]\n",
                                   name,
                                   fields.iter().map(|&(ref field, _)| field.number()).join(", ")));
        self.append_field_attributes(fq_message_name, oneof.name());
        self.push_indent();
        self.buf.push_str(&format!("pub {}: ::std::option::Option<{}>,\n", to_snake(oneof.name()), name));
    }

    fn append_oneof(&mut self,
                    msg_name: &str,
                    oneof: OneofDescriptorProto,
                    idx: i32,
                    fields: Vec<(FieldDescriptorProto, usize)>) {
        self.path.push(8);
        self.path.push(idx);
        self.append_doc();
        self.path.pop();
        self.path.pop();

        self.push_indent();
        self.buf.push_str("#[derive(Clone, Oneof, PartialEq)]\n");
        let oneof_name = format!("{}.{}", msg_name, oneof.name());
        self.append_type_attributes(&oneof_name);
        self.push_indent();
        self.buf.push_str("pub enum ");
        self.buf.push_str(&to_upper_camel(oneof.name()));
        self.buf.push_str(" {\n");

        self.path.push(2);
        self.depth += 1;
        for (field, idx) in fields {
            // TODO(danburkert/prost#19): support groups.
            let type_ = field.type_();
            if type_ == Type::Group { continue; }

            self.path.push(idx as i32);
            self.append_doc();
            self.path.pop();

            self.push_indent();
            let ty_tag = self.field_type_tag(&field);
            self.buf.push_str(&format!("#[prost({}, tag=\"{}\")]\n", ty_tag, field.number()));
            self.append_field_attributes(&oneof_name, field.name());

            self.push_indent();
            let ty = self.resolve_type(&field);

            let boxed = type_ == Type::Message
                     && self.message_graph.is_nested(field.type_name(), msg_name);

            debug!("    oneof: {:?}, type: {:?}, boxed: {}", field.name(), ty, boxed);

            if boxed {
                self.buf.push_str(&format!("{}(Box<{}>),\n", to_upper_camel(field.name()), ty));
            } else {
                self.buf.push_str(&format!("{}({}),\n", to_upper_camel(field.name()), ty));
            }
        }
        self.depth -= 1;
        self.path.pop();

        self.push_indent();
        self.buf.push_str("}\n");
    }

    fn location(&self) -> &Location {
        let idx = self.source_info
                      .location
                      .binary_search_by_key(&&self.path[..], |location| &location.path[..])
                      .unwrap();

        &self.source_info.location[idx]
    }

    fn append_doc(&mut self) {
        Comments::from_location(self.location()).append_with_indent(self.depth, &mut self.buf);
    }

    fn append_enum(&mut self, desc: EnumDescriptorProto) {
        debug!("  enum: {:?}", desc.name());

        // Skip Protobuf well-known types.
        let enum_name = &desc.name();
        let enum_values = &desc.value;
        let fq_enum_name = format!(".{}.{}", self.package, enum_name);
        if self.well_known_type(&fq_enum_name).is_some() { return; }

        self.append_doc();
        self.push_indent();
        self.buf.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Enumeration)]\n");
        self.append_type_attributes(&fq_enum_name);
        self.push_indent();
        self.buf.push_str("pub enum ");
        self.buf.push_str(&to_upper_camel(desc.name()));
        self.buf.push_str(" {\n");

        let mut numbers = HashSet::new();

        self.depth += 1;
        self.path.push(2);
        for (idx, value) in enum_values.into_iter().enumerate() {
            // Skip duplicate enum values. Protobuf allows this when the
            // 'allow_alias' option is set.
            if !numbers.insert(value.number()) {
                continue;
            }

            self.path.push(idx as i32);
            let stripped_prefix = if self.config.strip_enum_prefix {
                Some(to_upper_camel(&enum_name))
            } else {
                None
            };
            self.append_enum_value(&fq_enum_name, value, stripped_prefix);
            self.path.pop();
        }
        self.path.pop();
        self.depth -= 1;

        self.push_indent();
        self.buf.push_str("}\n");
    }

    fn append_enum_value(&mut self, fq_enum_name: &str, value: &EnumValueDescriptorProto, prefix_to_strip: Option<String>) {
        self.append_doc();
        self.append_field_attributes(fq_enum_name, &value.name());
        self.push_indent();
        let name = to_upper_camel(value.name());
        let name_unprefixed = match prefix_to_strip {
            Some(prefix) => {
                let is_prefixed = name.starts_with(&prefix) && name != prefix;
                if is_prefixed {
                    let prefix_len = prefix.len();
                    name[prefix_len..].to_string()
                }
                else { name }
            },
            None => name
        };
        self.buf.push_str(&name_unprefixed);
        self.buf.push_str(" = ");
        self.buf.push_str(&value.number().to_string());
        self.buf.push_str(",\n");
    }

    fn push_service(&mut self, service: ServiceDescriptorProto) {
        let name = service.name().to_owned();
        debug!("  service: {:?}", name);

        let comments = Comments::from_location(self.location());

        self.path.push(2);
        let methods = service.method
                             .into_iter()
                             .enumerate()
                             .map(|(idx, mut method)| {
                                 debug!("  method: {:?}", method.name());
                                 self.path.push(idx as i32);
                                 let comments = Comments::from_location(self.location());
                                 self.path.pop();

                                 let name = method.name.take().unwrap();
                                 let input_proto_type = method.input_type.take().unwrap();
                                 let output_proto_type = method.output_type.take().unwrap();
                                 let input_type = self.resolve_ident(&input_proto_type);
                                 let output_type = self.resolve_ident(&output_proto_type);
                                 let client_streaming = method.client_streaming();
                                 let server_streaming = method.server_streaming();

                                 Method {
                                     name: to_snake(&name),
                                     proto_name: name,
                                     comments,
                                     input_type,
                                     output_type,
                                     input_proto_type,
                                     output_proto_type,
                                     options: method.options.unwrap_or_default(),
                                     client_streaming,
                                     server_streaming,
                                 }
                             })
                             .collect();
        self.path.pop();

        let service = Service {
            name: to_upper_camel(&name),
            proto_name: name,
            package: self.package.clone(),
            comments,
            methods,
            options: service.options.unwrap_or_default(),
        };

        let buf = &mut self.buf;
        self.config.service_generator.as_mut().map(move |service_generator| service_generator.generate(service, buf));
    }

    fn push_indent(&mut self) {
        for _ in 0..self.depth {
            self.buf.push_str("    ");
        }
    }

    fn push_mod(&mut self, module: &str) {
        self.push_indent();
        self.buf.push_str("pub mod ");
        self.buf.push_str(&to_snake(module));
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
        match field.type_() {
            Type::Float => Cow::Borrowed("f32"),
            Type::Double => Cow::Borrowed("f64"),
            Type::Uint32 | Type::Fixed32 => Cow::Borrowed("u32"),
            Type::Uint64 | Type::Fixed64 => Cow::Borrowed("u64"),
            Type::Int32 | Type::Sfixed32 | Type::Sint32 | Type::Enum => Cow::Borrowed("i32"),
            Type::Int64 | Type::Sfixed64 | Type::Sint64 => Cow::Borrowed("i64"),
            Type::Bool => Cow::Borrowed("bool"),
            Type::String => Cow::Borrowed("String"),
            Type::Bytes => Cow::Borrowed("Vec<u8>"),
            Type::Group | Type::Message => {
                if let Some(ty) = self.well_known_type(field.type_name()) {
                    Cow::Borrowed(ty)
                } else {
                    Cow::Owned(self.resolve_ident(field.type_name()))
                }
            },
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
                  .chain(ident_path.map(to_snake))
                  .chain(Some(to_upper_camel(ident_type)).into_iter())
                  .join("::")
    }

    fn field_type_tag(&self, field: &FieldDescriptorProto) -> Cow<'static, str> {
        match field.type_() {
            Type::Float => Cow::Borrowed("float"),
            Type::Double => Cow::Borrowed("double"),
            Type::Int32 => Cow::Borrowed("int32"),
            Type::Int64 => Cow::Borrowed("int64"),
            Type::Uint32 => Cow::Borrowed("uint32"),
            Type::Uint64 => Cow::Borrowed("uint64"),
            Type::Sint32 => Cow::Borrowed("sint32"),
            Type::Sint64 => Cow::Borrowed("sint64"),
            Type::Fixed32 => Cow::Borrowed("fixed32"),
            Type::Fixed64 => Cow::Borrowed("fixed64"),
            Type::Sfixed32 => Cow::Borrowed("sfixed32"),
            Type::Sfixed64 => Cow::Borrowed("sfixed64"),
            Type::Bool => Cow::Borrowed("bool"),
            Type::String => Cow::Borrowed("string"),
            Type::Bytes => Cow::Borrowed("bytes"),
            Type::Group => Cow::Borrowed("group"),
            Type::Message => Cow::Borrowed("message"),
            Type::Enum => Cow::Owned(format!("enumeration={:?}", self.resolve_ident(field.type_name()))),
        }
    }

    fn map_value_type_tag(&self, field: &FieldDescriptorProto) -> Cow<'static, str> {
        match field.type_() {
            Type::Enum => Cow::Owned(format!("enumeration({})", self.resolve_ident(field.type_name()))),
            _ => self.field_type_tag(field),
        }
    }

    fn optional(&self, field: &FieldDescriptorProto) -> bool {
        if field.label() != Label::Optional {
            return false;
        }

        match field.type_() {
            Type::Message => true,
            _ => self.syntax == Syntax::Proto2,
        }
    }

    /// Returns the prost_types name for a well-known Protobuf type, or `None` if the provided
    /// message type is not a well-known type, or prost_types has been disabled.
    fn well_known_type(&self, fq_msg_type: &str) -> Option<&'static str> {
        if !self.config.prost_types { return None; }
        Some(match fq_msg_type {
            ".google.protobuf.BoolValue" => "bool",
            ".google.protobuf.BytesValue" => "::std::vec::Vec<u8>",
            ".google.protobuf.DoubleValue" => "f64",
            ".google.protobuf.Empty" => "()",
            ".google.protobuf.FloatValue" => "f32",
            ".google.protobuf.Int32Value" => "i32",
            ".google.protobuf.Int64Value" => "i64",
            ".google.protobuf.StringValue" => "::std::string::String",
            ".google.protobuf.UInt32Value" => "u32",
            ".google.protobuf.UInt64Value" => "u64",

            ".google.protobuf.Any" => "::prost_types::Any",
            ".google.protobuf.Api" => "::prost_types::Api",
            ".google.protobuf.DescriptorProto" => "::prost_types::DescriptorProto",
            ".google.protobuf.Duration" => "::prost_types::Duration",
            ".google.protobuf.Enum" => "::prost_types::Enum",
            ".google.protobuf.EnumDescriptorProto" => "::prost_types::EnumDescriptorProt",
            ".google.protobuf.EnumOptions" => "::prost_types::EnumOptions",
            ".google.protobuf.EnumValue" => "::prost_types::EnumValue",
            ".google.protobuf.EnumValueDescriptorProto" => "::prost_types::EnumValueDescriptorProto",
            ".google.protobuf.EnumValueOptions" => "::prost_types::EnumValueOptions",
            ".google.protobuf.ExtensionRangeOptions" => "::prost_types::ExtensionRangeOptions",
            ".google.protobuf.Field" => "::prost_types::Field",
            ".google.protobuf.FieldDescriptorProto" => "::prost_types::FieldDescriptorProto",
            ".google.protobuf.FieldMask" => "::prost_types::FieldMask",
            ".google.protobuf.FieldOptions" => "::prost_types::FieldOptions",
            ".google.protobuf.FileDescriptorProto" => "::prost_types::FileDescriptorProto",
            ".google.protobuf.FileDescriptorSet" => "::prost_types::FileDescriptorSet",
            ".google.protobuf.FileOptions" => "::prost_types::FileOptions",
            ".google.protobuf.GeneratedCodeInfo" => "::prost_types::GeneratedCodeInfo",
            ".google.protobuf.ListValue" => "::prost_types::ListValue",
            ".google.protobuf.MessageOptions" => "::prost_types::MessageOptions",
            ".google.protobuf.Method" => "::prost_types::Method",
            ".google.protobuf.MethodDescriptorProto" => "::prost_types::MethodDescriptorProto",
            ".google.protobuf.MethodOptions" => "::prost_types::MethodOptions",
            ".google.protobuf.Mixin" => "::prost_types::Mixin",
            ".google.protobuf.NullValue" => "::prost_types::NullValue",
            ".google.protobuf.OneofDescriptorProto" => "::prost_types::OneofDescriptorProto",
            ".google.protobuf.OneofOptions" => "::prost_types::OneofOptions",
            ".google.protobuf.Option" => "::prost_types::Option",
            ".google.protobuf.ServiceDescriptorProto" => "::prost_types::ServiceDescriptorProto",
            ".google.protobuf.ServiceOptions" => "::prost_types::ServiceOptions",
            ".google.protobuf.SourceCodeInfo" => "::prost_types::SourceCodeInfo",
            ".google.protobuf.SourceContext" => "::prost_types::SourceContext",
            ".google.protobuf.Struct" => "::prost_types::Struct",
            ".google.protobuf.Timestamp" => "::prost_types::Timestamp",
            ".google.protobuf.Type" => "::prost_types::Type",
            ".google.protobuf.UninterpretedOption" => "::prost_types::UninterpretedOption",
            ".google.protobuf.Value" => "::prost_types::Value",
            _ => return None,
        })
    }
}

/// Returns `true` if the repeated field type can be packed.
fn can_pack(field: &FieldDescriptorProto) -> bool {
        match field.type_() {
            Type::Float   | Type::Double  | Type::Int32    | Type::Int64    |
            Type::Uint32  | Type::Uint64  | Type::Sint32   | Type::Sint64   |
            Type::Fixed32 | Type::Fixed64 | Type::Sfixed32 | Type::Sfixed64 |
            Type::Bool    | Type::Enum => true,
            _ => false,
        }
}

/// Based on [`google::protobuf::UnescapeCEscapeString`][1]
/// [1]: https://github.com/google/protobuf/blob/3.3.x/src/google/protobuf/stubs/strutil.cc#L312-L322
fn unescape_c_escape_string(s: &str) -> Vec<u8> {
    let src = s.as_bytes();
    let len = src.len();
    let mut dst = Vec::new();

    let mut p = 0;

    while p < len {
        if src[p] != b'\\' {
            dst.push(src[p]);
            p += 1;
        } else {
            p += 1;
            if p == len {
                panic!("invalid c-escaped default binary value ({}): ends with '\'", s)
            }
            match src[p] {
                b'a' => {
                    dst.push(0x07);
                    p += 1;
                },
                b'b' => {
                    dst.push(0x08);
                    p += 1;
                },
                b'f' => {
                    dst.push(0x0C);
                    p += 1;
                },
                b'n' => {
                    dst.push(0x0A);
                    p += 1;
                },
                b'r' => {
                    dst.push(0x0D);
                    p += 1;
                },
                b't' => {
                    dst.push(0x09);
                    p += 1;
                },
                b'v' => {
                    dst.push(0x0B);
                    p += 1;
                },
                b'\\' => {
                    dst.push(0x5C);
                    p += 1;
                },
                b'?' => {
                    dst.push(0x3F);
                    p += 1;
                },
                b'\'' => {
                    dst.push(0x27);
                    p += 1;
                },
                b'"' => {
                    dst.push(0x22);
                    p += 1;
                },
                b'0'...b'7' => {
                    eprintln!("another octal: {}, offset: {}", s, &s[p..]);
                    let mut octal = 0;
                    for _ in 0..3 {
                        if p < len && src[p] >= b'0' && src[p] <= b'7' {
                            eprintln!("\toctal: {}", octal);
                            octal = octal * 8 + (src[p] - b'0');
                            p += 1;
                        } else {
                            break;
                        }
                    }
                    dst.push(octal);
                },
                b'x' | b'X' => {
                    if p + 2 > len {
                        panic!("invalid c-escaped default binary value ({}): incomplete hex value", s)
                    }
                    match u8::from_str_radix(&s[p+1..p+3], 16) {
                        Ok(b) => dst.push(b),
                        _ => panic!("invalid c-escaped default binary value ({}): invalid hex value", &s[p..p+2]),
                    }
                    p += 3;
                },
                _ => {
                    panic!("invalid c-escaped default binary value ({}): invalid escape", s)
                },
            }
        }
    }
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unescape_c_escape_string() {
        assert_eq!(&b"hello world"[..], &unescape_c_escape_string("hello world")[..]);

        assert_eq!(&b"\0"[..], &unescape_c_escape_string(r#"\0"#)[..]);

        assert_eq!(&[0o012, 0o156], &unescape_c_escape_string(r#"\012\156"#)[..]);
        assert_eq!(&[0x01, 0x02], &unescape_c_escape_string(r#"\x01\x02"#)[..]);

        assert_eq!(&b"\0\x01\x07\x08\x0C\n\r\t\x0B\\\'\"\xFE"[..],
                   &unescape_c_escape_string(r#"\0\001\a\b\f\n\r\t\v\\\'\"\xfe"#)[..]);
    }
}
