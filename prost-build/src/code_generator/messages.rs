use super::*;

mod oneof;

type OneofFields = MultiMap<i32, (FieldDescriptorProto, usize)>;
type MapTypes = HashMap<String, (FieldDescriptorProto, FieldDescriptorProto)>;

impl CodeGenerator<'_> {
    pub(super) fn push_messages(&mut self, message_types: Vec<DescriptorProto>) {
        self.path.push(FileDescriptorProtoLocations::MESSAGE_TYPE);
        for (idx, message) in message_types.into_iter().enumerate() {
            self.path.push(idx as i32);
            if let Some(resolved_message) = self.resolve_message(message) {
                self.buf.push_str(&resolved_message.to_string());
            }
            self.path.pop();
        }
        self.path.pop();
    }

    fn resolve_messages(
        &mut self,
        nested_types: Vec<(DescriptorProto, usize)>,
    ) -> Vec<TokenStream> {
        let mut messages = Vec::with_capacity(nested_types.len());

        self.path.push(3);
        for (nested_type, idx) in nested_types {
            self.path.push(idx as i32);
            if let Some(message) = self.resolve_message(nested_type) {
                messages.push(message);
            }
            self.path.pop();
        }
        self.path.pop();

        messages
    }

    fn resolve_message(&mut self, message: DescriptorProto) -> Option<TokenStream> {
        debug!("  message: {:?}", message.name());

        let message_name = message.name().to_string();
        let fq_message_name =
            FullyQualifiedName::new(&self.package, &self.type_path, &message_name);

        // Skip external types.
        if self.extern_paths.resolve_ident(&fq_message_name).is_some() {
            return None;
        }

        // Split the nested message types into a vector of normal nested message types, and a map
        // of the map field entry types. The path index of the nested message types is preserved so
        // that comments can be retrieved.
        type NestedTypes = Vec<(DescriptorProto, usize)>;
        let (nested_types, map_types): (NestedTypes, MapTypes) = message
            .nested_type
            .into_iter()
            .enumerate()
            .partition_map(|(idx, nested_type)| {
                if nested_type
                    .options
                    .as_ref()
                    .and_then(|options| options.map_entry)
                    .unwrap_or(false)
                {
                    let key = nested_type.field[0].clone();
                    let value = nested_type.field[1].clone();
                    assert_eq!("key", key.name());
                    assert_eq!("value", value.name());
                    Either::Right((
                        fq_message_name
                            .join(nested_type.name())
                            .as_ref()
                            .to_string(),
                        (key, value),
                    ))
                } else {
                    Either::Left((nested_type, idx))
                }
            });

        // Split the fields into a vector of the normal fields, and oneof fields.
        // Path indexes are preserved so that comments can be retrieved.
        type Fields = Vec<(FieldDescriptorProto, usize)>;
        let (fields, oneof_fields): (Fields, OneofFields) = message
            .field
            .into_iter()
            .enumerate()
            .partition_map(|(idx, field)| {
                if field.proto3_optional.unwrap_or(false) {
                    Either::Left((field, idx))
                } else if let Some(oneof_index) = field.oneof_index {
                    Either::Right((oneof_index, (field, idx)))
                } else {
                    Either::Left((field, idx))
                }
            });

        let documentation = self.resolve_docs(&fq_message_name, None);
        let resolved_fields = self.resolve_message_fields(&fields, &map_types, &fq_message_name);
        let resolved_oneof_fields = self.resolve_oneof_fields(
            &message.oneof_decl,
            &oneof_fields,
            &message_name,
            &fq_message_name,
        );

        let ident = to_syn_ident(&to_upper_camel(&message_name));

        let nested = self.recursive_nested(
            &message_name,
            message.enum_type,
            nested_types,
            oneof_fields,
            &message.oneof_decl,
            &fq_message_name,
        );

        let maybe_type_name = self
            .config
            .enable_type_names
            .then_some(self.resolve_type_name(&message_name, &fq_message_name));

        let type_attributes = self.config.type_attributes.get(fq_message_name.as_ref());
        let message_attributes = self.config.message_attributes.get(fq_message_name.as_ref());

        let prost_path = self.prost_type_path("Message");
        let maybe_skip_debug = self.resolve_skip_debug(&fq_message_name);

        Some(quote! {
            #(#documentation)*
            #(#(#type_attributes)*)*
            #(#(#message_attributes)*)*
            #[allow(clippy::derive_partial_eq_without_eq)]
            #[derive(Clone, PartialEq, #prost_path)]
            #maybe_skip_debug
            pub struct #ident {
                #(#resolved_fields,)*
                #(#resolved_oneof_fields,)*
            }

            #nested

            #maybe_type_name
        })
    }

    fn recursive_nested(
        &mut self,
        message_name: &str,
        enum_type: Vec<EnumDescriptorProto>,
        nested_types: Vec<(DescriptorProto, usize)>,
        oneof_fields: OneofFields,
        oneof_declarations: &[OneofDescriptorProto],
        fq_message_name: &FullyQualifiedName,
    ) -> Option<TokenStream> {
        if !enum_type.is_empty() || !nested_types.is_empty() || !oneof_fields.is_empty() {
            let comment = syn::Attribute::parse_outer
                .parse_str(&format!(
                    "/// Nested message and enum types in `{}`.",
                    message_name
                ))
                .expect("unable to parse comment");

            let ident = to_syn_ident(&to_snake(message_name));
            self.type_path.push(message_name.to_string());

            let resolved_messages = self.resolve_messages(nested_types);
            let resolved_enums = self.resolve_enums(enum_type);
            let resolved_oneofs =
                self.resolve_oneofs(oneof_declarations, oneof_fields, fq_message_name);

            self.type_path.pop();

            Some(quote! {
                #(#comment)*
                pub mod #ident {
                    #(#resolved_messages)*
                    #(#resolved_enums)*
                    #(#resolved_oneofs)*
                }
            })
        } else {
            None
        }
    }

    fn resolve_message_fields(
        &mut self,
        fields: &[(FieldDescriptorProto, usize)],
        map_types: &MapTypes,
        fq_message_name: &FullyQualifiedName,
    ) -> Vec<TokenStream> {
        let mut resolved_fields = Vec::with_capacity(fields.len());

        self.path.push(2);
        for (field, idx) in fields {
            self.path.push(*idx as i32);

            let field = match field
                .type_name
                .as_ref()
                .and_then(|type_name| map_types.get(type_name))
            {
                Some((key, value)) => self.resolve_map_field(fq_message_name, field, key, value),
                None => self.resolve_field(fq_message_name, field),
            };

            resolved_fields.push(field);
            self.path.pop();
        }
        self.path.pop();

        resolved_fields
    }

    fn resolve_type_name(
        &mut self,
        message_name: &str,
        fq_message_name: &FullyQualifiedName,
    ) -> TokenStream {
        let name_path = self.prost_type_path("Name");
        let message_name_syn = to_syn_type(message_name);
        let package_name = &self.package;
        let string_path = self.prost_type_path("alloc::string::String");
        let fully_qualified_name =
            FullyQualifiedName::new(&self.package, &self.type_path, message_name);
        let domain_name = self
            .config
            .type_name_domains
            .get_first(fq_message_name.as_ref())
            .map_or("", |name| name.as_str());

        let fq_name_str = fully_qualified_name.as_ref().trim_start_matches('.');
        let type_url = format!("{}/{}", domain_name, fq_name_str);

        quote! {
            impl #name_path for #message_name_syn {
                const NAME: &'static str = #message_name;
                const PACKAGE: &'static str = #package_name;

                fn full_name() -> #string_path { #fq_name_str.into() }
                fn type_url() -> #string_path { #type_url.into() }
            }
        }
    }

    fn resolve_field(
        &self,
        fq_message_name: &FullyQualifiedName,
        field: &FieldDescriptorProto,
    ) -> TokenStream {
        let type_ = field.r#type();
        let repeated = field.label == Some(Label::Repeated as i32);
        let optional = self.optional(field);
        let ty = self.resolve_type(field, fq_message_name);
        let boxed = !repeated && self.should_box_field(field, fq_message_name, fq_message_name);

        debug!(
            "    field: {:?}, type: {:?}, boxed: {}",
            field.name(),
            ty,
            boxed
        );

        let documentation = self.resolve_docs(fq_message_name, Some(field.name()));
        let maybe_deprecated = field
            .options
            .as_ref()
            .is_some_and(FieldOptions::deprecated)
            .then_some(quote! { #[deprecated] });
        let field_type_attr = to_syn_attribute_meta(&match type_ {
            Type::Bytes => {
                let bytes_type = self
                    .config
                    .bytes_type
                    .get_first_field(fq_message_name, field.name())
                    .copied()
                    .unwrap_or_default();

                Cow::from(format!(
                    "{}=\"{}\"",
                    self.field_type_tag(field),
                    bytes_type.annotation()
                ))
            }
            _ => self.field_type_tag(field),
        });
        let maybe_label = {
            match field.label() {
                Label::Optional => optional.then_some(quote! { optional, }),
                Label::Required => Some(quote! { required, }),
                Label::Repeated => Some(
                    match can_pack(field)
                        && !field
                            .options
                            .as_ref()
                            .map_or(self.syntax == Syntax::Proto3, |options| options.packed())
                    {
                        true => quote! { repeated, packed="false", },
                        false => quote! { repeated, },
                    },
                ),
            }
        };
        let maybe_boxed = boxed.then_some(quote! { boxed, });
        let field_number_string = field.number().to_string();
        let maybe_default = field.default_value.as_ref().map(|default| {
            let default_value = match type_ {
                Type::Bytes => {
                    let mut bytes_string = String::new();
                    bytes_string.push_str("b\\\"");
                    for b in unescape_c_escape_string(default) {
                        bytes_string.extend(
                            ascii::escape_default(b).flat_map(|c| (c as char).escape_default()),
                        );
                    }
                    bytes_string.push_str("\\\"");
                    bytes_string
                }
                Type::Enum => {
                    let mut enum_value = to_upper_camel(default);
                    if self.config.strip_enum_prefix {
                        let enum_type = field
                            .type_name
                            .as_ref()
                            .and_then(|ty| ty.split('.').last())
                            .expect("field type not fully qualified");

                        enum_value = strip_enum_prefix(&to_upper_camel(enum_type), &enum_value)
                    }

                    enum_value
                }
                _ => default.escape_default().to_string(),
            };
            to_syn_attribute_meta_value(&format!("default=\"{}\"", default_value))
        });

        let field_attributes = self.resolve_field_attributes(fq_message_name, field.name());
        let field_identifier = to_syn_ident(&to_snake(field.name()));

        let maybe_wrapped = if repeated {
            Some(self.prost_type_path("alloc::vec::Vec"))
        } else if optional {
            Some(to_syn_type_path("::core::option::Option"))
        } else {
            None
        };
        let maybe_boxed_type = boxed.then_some(self.prost_type_path("alloc::boxed::Box"));

        let inner_field_type = to_syn_type(&ty);

        let field_type = match (maybe_wrapped, &maybe_boxed_type) {
            (Some(wrapper), Some(boxed)) => quote! { #wrapper<#boxed<#inner_field_type>> },
            (Some(wrapper), None) => quote! { #wrapper<#inner_field_type> },
            (None, Some(boxed)) => quote! { #boxed<#inner_field_type> },
            (None, None) => quote! { #inner_field_type },
        };

        quote! {
            #(#documentation)*
            #maybe_deprecated
            #[prost(#field_type_attr, #maybe_label #maybe_boxed tag=#field_number_string, #maybe_default)]
            #field_attributes
            pub #field_identifier: #field_type
        }
    }

    fn resolve_map_field(
        &mut self,
        fq_message_name: &FullyQualifiedName,
        field: &FieldDescriptorProto,
        key: &FieldDescriptorProto,
        value: &FieldDescriptorProto,
    ) -> TokenStream {
        let key_ty = self.resolve_type(key, fq_message_name);
        let value_ty = self.resolve_type(value, fq_message_name);

        debug!(
            "    map field: {:?}, key type: {:?}, value type: {:?}",
            field.name(),
            key_ty,
            value_ty
        );

        let documentation = self.resolve_docs(fq_message_name, Some(field.name()));
        let map_type = self
            .config
            .map_type
            .get_first_field(fq_message_name, field.name())
            .copied()
            .unwrap_or_default();
        let key_tag = self.field_type_tag(key);
        let value_tag = self.map_value_type_tag(value);
        let meta_name_value = to_syn_attribute_meta_value(&format!(
            "{}=\"{}, {}\"",
            map_type.annotation(),
            key_tag,
            value_tag
        ));
        let field_number_string = field.number().to_string();
        let field_attributes = self.resolve_field_attributes(fq_message_name, field.name());
        let field_name_syn = to_syn_ident(&to_snake(field.name()));
        let map_rust_type = to_syn_type_path(map_type.rust_type());
        let key_rust_type = to_syn_type_path(&key_ty);
        let value_rust_type = to_syn_type_path(&value_ty);

        quote! {
            #(#documentation)*
            #[prost(#meta_name_value, tag=#field_number_string)]
            #field_attributes
            pub #field_name_syn: #map_rust_type<#key_rust_type, #value_rust_type>
        }
    }
}

// Helpers
impl CodeGenerator<'_> {
    fn resolve_skip_debug(&self, fq_message_name: &FullyQualifiedName) -> Option<TokenStream> {
        self.should_skip_debug(fq_message_name)
            .then_some(quote! { #[prost(skip_debug)] })
    }

    fn should_box_field(
        &self,
        field: &FieldDescriptorProto,
        fq_message_name: &FullyQualifiedName,
        first_field: &FullyQualifiedName,
    ) -> bool {
        ((matches!(field.r#type(), Type::Message | Type::Group))
            && self
                .message_graph
                .is_nested(field.type_name(), fq_message_name.as_ref()))
            || (self
                .config
                .boxed
                .get_first_field(first_field, field.name())
                .is_some())
    }

    // TODO: to syn::Type
    fn resolve_type(
        &self,
        field: &FieldDescriptorProto,
        fq_message_name: &FullyQualifiedName,
    ) -> String {
        match field.r#type() {
            Type::Float => String::from("f32"),
            Type::Double => String::from("f64"),
            Type::Uint32 | Type::Fixed32 => String::from("u32"),
            Type::Uint64 | Type::Fixed64 => String::from("u64"),
            Type::Int32 | Type::Sfixed32 | Type::Sint32 | Type::Enum => String::from("i32"),
            Type::Int64 | Type::Sfixed64 | Type::Sint64 => String::from("i64"),
            Type::Bool => String::from("bool"),
            Type::String => format!("{}::alloc::string::String", self.resolve_prost_path()),
            Type::Bytes => self
                .config
                .bytes_type
                .get_first_field(fq_message_name, field.name())
                .copied()
                .unwrap_or_default()
                .rust_type()
                .to_owned(),
            Type::Group | Type::Message => {
                self.resolve_ident(&FullyQualifiedName::from_type_name(field.type_name()))
            }
        }
    }

    fn map_value_type_tag(&self, field: &FieldDescriptorProto) -> Cow<'static, str> {
        match field.r#type() {
            Type::Enum => Cow::Owned(format!(
                "enumeration({})",
                self.resolve_ident(&FullyQualifiedName::from_type_name(field.type_name()))
            )),
            _ => self.field_type_tag(field),
        }
    }

    fn field_type_tag(&self, field: &FieldDescriptorProto) -> Cow<'static, str> {
        match field.r#type() {
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
            Type::Enum => Cow::Owned(format!(
                "enumeration=\"{}\"",
                self.resolve_ident(&FullyQualifiedName::from_type_name(field.type_name()))
            )),
        }
    }

    fn optional(&self, field: &FieldDescriptorProto) -> bool {
        if field.proto3_optional.unwrap_or(false) {
            return true;
        }

        if field.label() != Label::Optional {
            return false;
        }

        match field.r#type() {
            Type::Message => true,
            _ => self.syntax == Syntax::Proto2,
        }
    }
}

/// Returns `true` if the repeated field type can be packed.
fn can_pack(field: &FieldDescriptorProto) -> bool {
    matches!(
        field.r#type(),
        Type::Float
            | Type::Double
            | Type::Int32
            | Type::Int64
            | Type::Uint32
            | Type::Uint64
            | Type::Sint32
            | Type::Sint64
            | Type::Fixed32
            | Type::Fixed64
            | Type::Sfixed32
            | Type::Sfixed64
            | Type::Bool
            | Type::Enum
    )
}
