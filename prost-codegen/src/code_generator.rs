use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use itertools::{Either, Itertools};
use multimap::MultiMap;

use ast::{
    Comments,
    Method,
    Service,
};
use google::protobuf::{
    DescriptorProto,
    EnumDescriptorProto,
    EnumValueDescriptorProto,
    FieldDescriptorProto,
    FileDescriptorProto,
    OneofDescriptorProto,
    ServiceDescriptorProto,
    SourceCodeInfo,
};
use google::protobuf::field_descriptor_proto::{Label, Type};
use google::protobuf::source_code_info::Location;
use ident::{
    camel_to_snake,
    match_field,
    snake_to_upper_camel,
};
use message_graph::MessageGraph;
use Config;
use Module;

pub fn module(file: &FileDescriptorProto) -> Module {
    file.package()
        .split('.')
        .filter(|s| !s.is_empty())
        .map(camel_to_snake)
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
        debug!("\tmessage: {:?}", message.name());

        // Split the nested message types into a vector of normal nested message types, and a map
        // of the map field entry types. The path index of the nested message types is preserved so
        // that comments can be retrieved.
        let message_name = message.name.as_ref().expect("message name");
        let fq_message_name = format!(".{}.{}", self.package, message_name);
        let (nested_types, map_types): (Vec<(DescriptorProto, usize)>, HashMap<String, (FieldDescriptorProto, FieldDescriptorProto)>) =
            message.nested_type.into_iter().enumerate().partition_map(|(idx, nested_type)| {
                if nested_type.options.as_ref().and_then(|options| options.map_entry).unwrap_or(false) {
                    let key = nested_type.field[0].clone();
                    let value = nested_type.field[1].clone();
                    assert_eq!("key", key.name.as_ref().expect("key name"));
                    assert_eq!("value", value.name.as_ref().expect("value name"));

                    let name = format!("{}.{}",
                                       fq_message_name,
                                       nested_type.name.as_ref().expect("nested type name"));
                    Either::Right((name, (key, value)))
                } else {
                    Either::Left((nested_type, idx))
                }
        });

        // Split the fields into a vector of the normal fields, and oneof fields.
        // Path indexes are preserved so that comments can be retrieved.
        let (fields, mut oneof_fields): (Vec<(FieldDescriptorProto, usize)>, MultiMap<i32, (FieldDescriptorProto, usize)>) =
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
        self.buf.push_str("#[derive(Clone, Debug, PartialEq, Message)]\n");
        self.push_indent();
        self.buf.push_str("pub struct ");
        self.buf.push_str(&message_name);
        self.buf.push_str(" {\n");

        self.depth += 1;
        self.path.push(2);
        for (field, idx) in fields.into_iter() {
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

    fn append_field(&mut self, msg_name: &str, field: FieldDescriptorProto) {
        // TODO(danburkert/prost#19): support groups.
        if field.type_().unwrap() == Type::TypeGroup { return; }

        let repeated = field.label == Some(Label::LabelRepeated as i32);
        let optional = self.optional(&field);
        let ty = self.resolve_type(&field);

        let boxed = !repeated
                 && field.type_().unwrap() == Type::TypeMessage
                 && self.message_graph.is_nested(field.type_name(), msg_name);

        debug!("\t\tfield: {:?}, type: {:?}", field.name(), ty);

        self.append_doc();
        self.push_indent();
        self.buf.push_str("#[prost(");
        let type_tag = self.field_type_tag(&field);
        self.buf.push_str(&type_tag);

        match field.label().expect("unknown label") {
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
        self.buf.push_str("\")]\n");
        self.push_indent();
        self.buf.push_str("pub ");
        self.buf.push_str(&camel_to_snake(field.name()));
        self.buf.push_str(": ");
        if repeated { self.buf.push_str("Vec<"); }
        else if optional { self.buf.push_str("Option<"); }
        if boxed { self.buf.push_str("Box<"); }
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

        debug!("\t\tmap field: {:?}, key type: {:?}, value type: {:?}",
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
                                   camel_to_snake(field.name()), rust_ty, key_ty, value_ty));
    }

    fn append_oneof_field(&mut self,
                          message_name: &str,
                          oneof: &OneofDescriptorProto,
                          fields: &[(FieldDescriptorProto, usize)]) {
        let name = format!("{}::{}",
                           camel_to_snake(message_name),
                           snake_to_upper_camel(oneof.name()));
        self.append_doc();
        self.push_indent();
        self.buf.push_str(&format!("#[prost(oneof=\"{}\", tags=\"{}\")]\n",
                                   name,
                                   fields.iter().map(|&(ref field, _)| field.number()).join(", ")));
        self.push_indent();
        self.buf.push_str(&format!("pub {}: Option<{}>,\n", camel_to_snake(oneof.name()), name));
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
        self.buf.push_str("#[derive(Clone, Debug, Oneof, PartialEq)]\n");
        self.push_indent();
        self.buf.push_str("pub enum ");
        self.buf.push_str(&snake_to_upper_camel(oneof.name()));
        self.buf.push_str(" {\n");

        self.path.push(2);
        self.depth += 1;
        for (field, idx) in fields {
            // TODO(danburkert/prost#19): support groups.
            if field.type_().unwrap() == Type::TypeGroup { continue; }

            self.path.push(idx as i32);
            self.append_doc();
            self.path.pop();

            self.push_indent();
            let ty_tag = self.field_type_tag(&field);
            self.buf.push_str(&format!("#[prost({}, tag=\"{}\")]\n", ty_tag, field.number()));

            self.push_indent();
            let ty = self.resolve_type(&field);
            self.buf.push_str(&format!("{}({}),\n", snake_to_upper_camel(field.name()), ty));
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

        if let Some(ref comments) = location.leading_comments {
            for line in comments.lines() {
                for _ in 0..self.depth {
                    self.buf.push_str("    ");
                }
                self.buf.push_str("///");
                self.buf.push_str(line);
                self.buf.push_str("\n");
            }
        }
        if let Some(ref comments) = location.trailing_comments {
            for line in comments.lines() {
                for _ in 0..self.depth {
                    self.buf.push_str("    ");
                }
                self.buf.push_str("///");
                self.buf.push_str(line);
                self.buf.push_str("\n");
            }
        }
    }

    fn append_enum(&mut self, desc: EnumDescriptorProto) {
        debug!("\tenum: {:?}", desc.name());

        self.append_doc();
        self.push_indent();
        self.buf.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq, Enumeration)]\n");
        self.push_indent();
        self.buf.push_str("pub enum ");
        self.buf.push_str(desc.name());
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
        self.buf.push_str(&snake_to_upper_camel(value.name()));
        self.buf.push_str(" = ");
        self.buf.push_str(&value.number().to_string());
        self.buf.push_str(",\n");
    }

    fn unpack_service(&mut self, service: ServiceDescriptorProto) -> Service {
        let name = service.name().to_owned();
        debug!("\t service: {:?}", name);

        let comments = Comments::from_location(self.location());

        let methods = service.method
                              .into_iter()
                              .enumerate()
                              .map(|(idx, mut method)| {
                                  self.path.push(idx as i32);
                                  let comments = Comments::from_location(self.location());
                                  self.path.pop();

                                  let name = method.name.take().unwrap();
                                  let input_proto_type = method.input_type.take().unwrap();
                                  let output_proto_type = method.output_type.take().unwrap();
                                  let input_type = self.resolve_ident(&input_proto_type);
                                  let output_type = self.resolve_ident(&output_proto_type);

                                  Method {
                                      name,
                                      comments,
                                      input_type,
                                      input_proto_type,
                                      output_type,
                                      output_proto_type
                                  }
                              })
                              .collect();

        Service {
            name,
            comments,
            methods
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
        match field.type_().expect("unknown field type") {
            Type::TypeFloat => Cow::Borrowed("f32"),
            Type::TypeDouble => Cow::Borrowed("f64"),
            Type::TypeUint32 | Type::TypeFixed32 => Cow::Borrowed("u32"),
            Type::TypeUint64 | Type::TypeFixed64 => Cow::Borrowed("u64"),
            Type::TypeInt32 | Type::TypeSfixed32 | Type::TypeSint32 => Cow::Borrowed("i32"),
            Type::TypeInt64 | Type::TypeSfixed64 | Type::TypeSint64 => Cow::Borrowed("i64"),
            Type::TypeBool => Cow::Borrowed("bool"),
            Type::TypeString => Cow::Borrowed("String"),
            Type::TypeBytes => Cow::Borrowed("Vec<u8>"),
            Type::TypeGroup | Type::TypeMessage => Cow::Owned(self.resolve_ident(field.type_name())),
            Type::TypeEnum => Cow::Borrowed("i32"),
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

    fn field_type_tag(&self, field: &FieldDescriptorProto) -> Cow<'static, str> {
        match field.type_().expect("unknown field type") {
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
        match field.type_().expect("unknown field type") {
            Type::TypeEnum => Cow::Owned(format!("enumeration({})", self.resolve_ident(field.type_name()))),
            _ => self.field_type_tag(field),
        }
    }

    fn optional(&self, field: &FieldDescriptorProto) -> bool {
        if field.label().expect("unknown label") != Label::LabelOptional {
            return false;
        }

        match field.type_().expect("unknown field type") {
            Type::TypeMessage => true,
            _ => self.syntax == Syntax::Proto2,
        }
    }
}

fn can_pack(field: &FieldDescriptorProto) -> bool {
        match field.type_().expect("unknown field type") {
            Type::TypeFloat   | Type::TypeDouble  | Type::TypeInt32    | Type::TypeInt64    |
            Type::TypeUint32  | Type::TypeUint64  | Type::TypeSint32   | Type::TypeSint64   |
            Type::TypeFixed32 | Type::TypeFixed64 | Type::TypeSfixed32 | Type::TypeSfixed64 |
            Type::TypeBool    | Type::TypeEnum => true,
            _ => false,
        }
}
