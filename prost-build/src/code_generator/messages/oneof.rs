use super::*;

impl CodeGenerator<'_> {
    pub(super) fn resolve_oneofs(
        &mut self,
        oneof_declarations: &[OneofDescriptorProto],
        mut oneof_fields: OneofFields,
        fq_message_name: &FullyQualifiedName,
    ) -> Vec<TokenStream> {
        let mut oneofs = Vec::with_capacity(oneof_declarations.len());

        for (idx, oneof) in oneof_declarations.iter().enumerate() {
            let idx = idx as i32;
            // optional fields create a synthetic oneof that we want to skip
            let fields = match oneof_fields.remove(&idx) {
                Some(fields) => fields,
                None => continue,
            };
            oneofs.push(self.append_oneof(fq_message_name, oneof, idx, fields));
        }

        oneofs
    }

    pub(super) fn resolve_oneof_fields(
        &mut self,
        oneof_declarations: &[OneofDescriptorProto],
        oneof_fields: &OneofFields,
        message_name: &str,
        fq_message_name: &FullyQualifiedName,
    ) -> Vec<TokenStream> {
        let mut resolved_onefields = Vec::with_capacity(oneof_declarations.len());

        self.path.push(DescriptorLocations::ONEOF_DECL);
        for (idx, oneof) in oneof_declarations.iter().enumerate() {
            let idx = idx as i32;

            let fields = match oneof_fields.get_vec(&idx) {
                Some(fields) => fields,
                None => continue,
            };

            self.path.push(idx);

            resolved_onefields.push(self.resolve_oneof_field(
                message_name,
                fq_message_name,
                oneof,
                fields,
            ));

            self.path.pop();
        }
        self.path.pop();

        resolved_onefields
    }

    fn resolve_oneof_field(
        &mut self,
        message_name: &str,
        fq_message_name: &FullyQualifiedName,
        oneof: &OneofDescriptorProto,
        fields: &[(FieldDescriptorProto, usize)],
    ) -> TokenStream {
        let documentation = self.resolve_docs(fq_message_name, None);
        let oneof_name = format!(
            "{}::{}",
            to_snake(message_name),
            to_upper_camel(oneof.name())
        );
        let tags = fields.iter().map(|(field, _)| field.number()).join(", ");
        let field_attributes = self.resolve_field_attributes(fq_message_name, oneof.name());
        let field_name = to_snake(oneof.name()).parse_syn::<syn::Ident>();
        let oneof_type_name = oneof_name.parse_syn::<syn::TypePath>();

        quote! {
            #(#documentation)*
            #[prost(oneof=#oneof_name, tags=#tags)]
            #field_attributes
            pub #field_name: ::core::option::Option<#oneof_type_name>
        }
    }

    fn append_oneof(
        &mut self,
        fq_message_name: &FullyQualifiedName,
        oneof: &OneofDescriptorProto,
        idx: i32,
        fields: Vec<(FieldDescriptorProto, usize)>,
    ) -> TokenStream {
        self.path.push(DescriptorLocations::ONEOF_DECL);
        self.path.push(idx);
        let documentation = self.resolve_docs(fq_message_name, None);
        self.path.pop();
        self.path.pop();

        let oneof_name = fq_message_name.join(oneof.name());
        let enum_attributes = self.resolve_enum_attributes(&oneof_name);
        let maybe_skip_debug = self.resolve_skip_debug(fq_message_name);
        let enum_name = to_upper_camel(oneof.name()).parse_syn::<syn::Ident>();
        let variants = self.oneof_variants(&fields, fq_message_name, &oneof_name);

        let one_of_path = self.prost_type_path("Oneof");
        quote! {
            #(#documentation)*
            #enum_attributes
            #[allow(clippy::derive_partial_eq_without_eq)]
            #[derive(Clone, PartialEq, #one_of_path)]
            #maybe_skip_debug
            pub enum #enum_name {
                #(#variants,)*
            }
        }
    }

    fn oneof_variants(
        &mut self,
        fields: &[(FieldDescriptorProto, usize)],
        fq_message_name: &FullyQualifiedName,
        oneof_name: &FullyQualifiedName,
    ) -> Vec<TokenStream> {
        let mut variants = Vec::with_capacity(fields.len());

        self.path.push(DescriptorLocations::FIELD);
        for (field, idx) in fields {
            self.path.push((*idx).try_into().expect("idx overflow"));
            let documentation = self.resolve_docs(fq_message_name, Some(field.name()));
            self.path.pop();

            let ty_tag = self.field_type_tag(field).parse_syn::<syn::Meta>();
            let field_number_string = field.number().to_string();
            let field_attributes = self.resolve_field_attributes(oneof_name, field.name());
            let enum_variant = {
                let rust_type = self.resolve_type(field, fq_message_name);
                let type_path = rust_type.parse_syn::<syn::TypePath>();
                let field_name = to_upper_camel(field.name()).parse_syn::<syn::Ident>();

                let boxed = self.should_box_field(field, fq_message_name, oneof_name);

                debug!(
                    "    oneof: {}, type: {}, boxed: {}",
                    field.name(),
                    rust_type,
                    boxed
                );

                match boxed {
                    true => quote! {
                        #field_name(::prost::alloc::boxed::Box<#type_path>)
                    },
                    false => quote! {
                        #field_name(#type_path)
                    },
                }
            };

            variants.push(quote! {
                 #(#documentation)*
                 #[prost(#ty_tag, tag=#field_number_string)]
                 #field_attributes
                 #enum_variant
            });
        }
        self.path.pop();

        variants
    }
}
