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
    match_field,
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
    config: &'a Config,
    package: String,
    source_info: SourceCodeInfo,
    syntax: Syntax,
    message_graph: &'a MessageGraph,
    depth: u8,
    path: Vec<i32>,
    buf: &'a mut String,
}

impl <'a> CodeGenerator<'a> {
    pub fn generate(config: &Config,
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

        if let Some(ref service_generator) = code_gen.config.service_generator {
            code_gen.path.push(6);
            for (idx, service) in file.service.into_iter().enumerate() {
                code_gen.path.push(idx as i32);
                service_generator.generate(code_gen.unpack_service(service), &mut code_gen.buf);
                code_gen.path.pop();
            }
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

        self.buf.push_str("#[derive(Clone, Debug, PartialEq, Message");
        #[cfg(feature="serde-1")]
        {
            self.buf.push_str(", Serialize, Deserialize");
        }
        self.buf.push_str(")]\n");

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
            self.append_oneof_field(&message_name, oneof, oneof_fields.get_vec(&idx).unwrap());
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
                self.append_oneof(oneof, idx, oneof_fields.remove(&idx).unwrap());
            }

            self.pop_mod();
        }
    }

    fn append_field(&mut self, msg_name: &str, field: FieldDescriptorProto) {
        // TODO(danburkert/prost#19): support groups.
        let type_ = field.type_();
        if type_ == Type::TypeGroup { return; }

        let repeated = field.label == Some(Label::LabelRepeated as i32);
        let optional = self.optional(&field);
        let ty = self.resolve_type(&field);

        let boxed = !repeated
                 && type_ == Type::TypeMessage
                 && self.message_graph.is_nested(field.type_name(), msg_name);

        debug!("    field: {:?}, type: {:?}", field.name(), ty);

        self.append_doc();
        self.push_indent();
        self.buf.push_str("#[prost(");
        let type_tag = self.field_type_tag(&field);
        self.buf.push_str(&type_tag);

        match field.label() {
            Label::LabelOptional => if optional {
                self.buf.push_str(", optional");
            },
            Label::LabelRequired => self.buf.push_str(", required"),
            Label::LabelRepeated => {
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
            if type_ == Type::TypeBytes {
                self.buf.push_str("b\\\"");
                for b in unescape_c_escape_string(default) {
                    self.buf.extend(ascii::escape_default(b).flat_map(|c| (c as char).escape_default()));
                }
                self.buf.push_str("\\\"");
            } else if type_ == Type::TypeEnum {
                self.buf.push_str(&to_upper_camel(default));
            } else {
                // TODO: this is only correct if the Protobuf escaping matches Rust escaping. To be
                // safer, we should unescape the Protobuf string and re-escape it with the Rust
                // escaping mechanisms.
                self.buf.push_str(default);
            }
        }

        self.buf.push_str("\")]\n");
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
                            .any(|matcher| match_field(matcher, msg_name, field.name()));
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
        self.push_indent();
        self.buf.push_str(&format!("pub {}: ::std::collections::{}<{}, {}>,\n",
                                   to_snake(field.name()), rust_ty, key_ty, value_ty));
    }

    fn append_oneof_field(&mut self,
                          message_name: &str,
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
        self.push_indent();
        self.buf.push_str(&format!("pub {}: ::std::option::Option<{}>,\n", to_snake(oneof.name()), name));
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
        
        self.buf.push_str("#[derive(Clone, Debug, Oneof, PartialEq");
        #[cfg(feature="serde-1")]
        {
            self.buf.push_str(", Serialize, Deserialize");
        }
        self.buf.push_str(")]\n");

        self.push_indent();
        self.buf.push_str("pub enum ");
        self.buf.push_str(&to_upper_camel(oneof.name()));
        self.buf.push_str(" {\n");

        self.path.push(2);
        self.depth += 1;
        for (field, idx) in fields {
            // TODO(danburkert/prost#19): support groups.
            if field.type_() == Type::TypeGroup { continue; }

            self.path.push(idx as i32);
            self.append_doc();
            self.path.pop();

            self.push_indent();
            let ty_tag = self.field_type_tag(&field);
            self.buf.push_str(&format!("#[prost({}, tag=\"{}\")]\n", ty_tag, field.number()));

            self.push_indent();
            let ty = self.resolve_type(&field);
            self.buf.push_str(&format!("{}({}),\n", to_upper_camel(field.name()), ty));
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
        let fq_enum_name = format!(".{}.{}", self.package, desc.name());
        if self.well_known_type(&fq_enum_name).is_some() { return; }

        self.append_doc();
        self.push_indent();

        self.buf.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq, Enumeration");
        #[cfg(feature="serde-1")]
        {
            self.buf.push_str(", Serialize, Deserialize");
        }
        self.buf.push_str(")]\n");

        self.push_indent();
        self.buf.push_str("pub enum ");
        self.buf.push_str(&to_upper_camel(desc.name()));
        self.buf.push_str(" {\n");

        let mut numbers = HashSet::new();

        self.depth += 1;
        self.path.push(2);
        for (idx, value) in desc.value.into_iter().enumerate() {
            // Skip duplicate enum values. Protobuf allows this when the
            // 'allow_alias' option is set.
            if !numbers.insert(value.number()) {
                continue;
            }

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
        self.buf.push_str(&to_upper_camel(value.name()));
        self.buf.push_str(" = ");
        self.buf.push_str(&value.number().to_string());
        self.buf.push_str(",\n");
    }

    fn unpack_service(&mut self, service: ServiceDescriptorProto) -> Service {
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

        Service {
            name: to_upper_camel(&name),
            proto_name: name,
            package: self.package.clone(),
            comments,
            methods,
            options: service.options.unwrap_or_default(),
        }
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
            Type::TypeFloat => Cow::Borrowed("f32"),
            Type::TypeDouble => Cow::Borrowed("f64"),
            Type::TypeUint32 | Type::TypeFixed32 => Cow::Borrowed("u32"),
            Type::TypeUint64 | Type::TypeFixed64 => Cow::Borrowed("u64"),
            Type::TypeInt32 | Type::TypeSfixed32 | Type::TypeSint32 | Type::TypeEnum => Cow::Borrowed("i32"),
            Type::TypeInt64 | Type::TypeSfixed64 | Type::TypeSint64 => Cow::Borrowed("i64"),
            Type::TypeBool => Cow::Borrowed("bool"),
            Type::TypeString => Cow::Borrowed("String"),
            Type::TypeBytes => Cow::Borrowed("Vec<u8>"),
            Type::TypeGroup | Type::TypeMessage => {
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
            Type::TypeFloat => Cow::Borrowed("float"),
            Type::TypeDouble => Cow::Borrowed("double"),
            Type::TypeInt32 => Cow::Borrowed("int32"),
            Type::TypeInt64 => Cow::Borrowed("int64"),
            Type::TypeUint32 => Cow::Borrowed("uint32"),
            Type::TypeUint64 => Cow::Borrowed("uint64"),
            Type::TypeSint32 => Cow::Borrowed("sint32"),
            Type::TypeSint64 => Cow::Borrowed("sint64"),
            Type::TypeFixed32 => Cow::Borrowed("fixed32"),
            Type::TypeFixed64 => Cow::Borrowed("fixed64"),
            Type::TypeSfixed32 => Cow::Borrowed("sfixed32"),
            Type::TypeSfixed64 => Cow::Borrowed("sfixed64"),
            Type::TypeBool => Cow::Borrowed("bool"),
            Type::TypeString => Cow::Borrowed("string"),
            Type::TypeBytes => Cow::Borrowed("bytes"),
            Type::TypeGroup => Cow::Borrowed("group"),
            Type::TypeMessage => Cow::Borrowed("message"),
            Type::TypeEnum => Cow::Owned(format!("enumeration={:?}", self.resolve_ident(field.type_name()))),
        }
    }

    fn map_value_type_tag(&self, field: &FieldDescriptorProto) -> Cow<'static, str> {
        match field.type_() {
            Type::TypeEnum => Cow::Owned(format!("enumeration({})", self.resolve_ident(field.type_name()))),
            _ => self.field_type_tag(field),
        }
    }

    fn optional(&self, field: &FieldDescriptorProto) -> bool {
        if field.label() != Label::LabelOptional {
            return false;
        }

        match field.type_() {
            Type::TypeMessage => true,
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
            Type::TypeFloat   | Type::TypeDouble  | Type::TypeInt32    | Type::TypeInt64    |
            Type::TypeUint32  | Type::TypeUint64  | Type::TypeSint32   | Type::TypeSint64   |
            Type::TypeFixed32 | Type::TypeFixed64 | Type::TypeSfixed32 | Type::TypeSfixed64 |
            Type::TypeBool    | Type::TypeEnum => true,
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
                b'a' => dst.push(0x07),
                b'b' => dst.push(0x08),
                b'f' => dst.push(0x0C),
                b'n' => dst.push(0x0A),
                b'r' => dst.push(0x0D),
                b't' => dst.push(0x09),
                b'v' => dst.push(0x0B),
                b'\\' => dst.push(0x5C),
                b'?' => dst.push(0x3F),
                b'\'' => dst.push(0x27),
                b'"' => dst.push(0x22),
                b'0'...b'7' => {
                    if p + 3 > len {
                        eprintln!("p: {}, len: {}, src[p..]: {}", p, len, &s[p..]);
                        panic!("invalid c-escaped default binary value ({}): incomplete octal value", s)
                    }
                    match u8::from_str_radix(&s[p..p+3], 8) {
                        Ok(b) => dst.push(b),
                        _ => panic!("invalid c-escaped default binary value ({}): invalid octal value", s),
                    }
                    p += 3;
                },
                b'x' | b'X' => {
                    if p + 2 > len {
                        panic!("invalid c-escaped default binary value ({}): incomplete hex value", s)
                    }
                    match u8::from_str_radix(&s[p..p+2], 16) {
                        Ok(b) => dst.push(b),
                        _ => panic!("invalid c-escaped default binary value ({}): invalid hex value", s),
                    }
                    p += 2;
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
    }
}
