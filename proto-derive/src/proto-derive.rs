// The `quote!` macro requires deep recursion.
#![recursion_limit = "192"]

extern crate proc_macro;
extern crate proto;
extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

use proto::field::{MAX_TAG, MIN_TAG};

enum FieldKind {
    Field,
    FixedField,
    SignedField,
}

struct Field {
    field: syn::Field,
    kind: FieldKind,
    tag: u32,
}

impl Field {
    fn extract(field: syn::Field) -> Option<Field> {
        let mut tag = None;
        let mut fixed = false;
        let mut signed = false;
        let mut ignore = false;

        {
            // Get the metadata items belonging to 'proto' list attributes (e.g. #[proto(foo, bar="baz")]).
            let proto_items = field.attrs.iter().flat_map(|attr| {
                match attr.value {
                    syn::MetaItem::List(ref ident, ref items) if ident == "proto" => items.into_iter(),
                    _ => [].into_iter(),
                }
            });


            for item in proto_items {
                match *item {
                    // Handle `#[proto(tag = 1)] or #[proto(tag = "1")]`.
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, syn::Lit::Int(value, _))) if name == "tag" => tag = Some(value),
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, syn::Lit::Str(ref value, _))) if name == "tag" => {
                        match value.parse() {
                            Ok(value) => tag = Some(value),
                            Err(..) => panic!("tag attribute value must be an integer"),
                        }
                    }

                    // Handle `#[proto(fixed)]` and `#[proto(fixed = false)].
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref name)) if name == "fixed" => fixed = true,
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, syn::Lit::Bool(value))) if name == "fixed" => fixed = value,

                    // Handle `#[proto(signed)]` and `#[proto(signed = false)]`.
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref name)) if name == "signed" => signed = true,
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, syn::Lit::Bool(value))) if name == "signed" => signed = value,

                    // Handle `#[proto(ignore)]` and `#[proto(ignore = false)]`.
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref name)) if name == "ignore" => ignore = true,
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, syn::Lit::Bool(value))) if name == "ignore" => ignore = value,

                    syn::NestedMetaItem::MetaItem(ref meta_item) => panic!("unknown proto field attribute item `{}`", meta_item.name()),
                    syn::NestedMetaItem::Literal(_) => panic!("unexpected literal in serde field attribute"),
                }
            }
        }

        let (tag, kind) = match (tag, fixed, signed, ignore) {
            (Some(_), _, _, true)           => panic!("ignored proto field must not have a tag attribute"),
            (None, _, _, false)             => panic!("proto field must have a tag attribute"),
            (None, true, _, true)           => panic!("ignored proto field must not be fixed"),
            (None, _, true, true)           => panic!("ignored proto field must not be signed"),
            (Some(_), true, true, false)    => panic!("proto field must not be fixed and signed"),
            (None, false, false, true)      => return None,
            (Some(tag), _, _, false) if tag > MAX_TAG as u64 => panic!("proto tag must be less than or equal to {}", MAX_TAG),
            (Some(tag), _, _, false) if tag < MIN_TAG as u64 => panic!("proto tag must be greater than or equal to {}", MIN_TAG),
            (Some(tag), false, false, false) => (tag as u32, FieldKind::Field),
            (Some(tag), true, false, false)  => (tag as u32, FieldKind::FixedField),
            (Some(tag), false, true, false)  => (tag as u32, FieldKind::SignedField),
        };

        Some(Field {
            field: field,
            kind: kind,
            tag: tag,
        })
    }
}

#[proc_macro_derive(Message, attributes(proto))]
pub fn message(input: TokenStream) -> TokenStream {
    let syn::DeriveInput { ident, generics, body, .. } =
        syn::parse_derive_input(&input.to_string()).expect("unable to parse message type");

    if !generics.lifetimes.is_empty() ||
       !generics.ty_params.is_empty() ||
       !generics.where_clause.predicates.is_empty() {
        panic!("Message may not be derived for generic type");
    }

    let fields = match body {
        syn::Body::Struct(syn::VariantData::Struct(fields)) => fields,
        syn::Body::Struct(syn::VariantData::Tuple(fields)) => fields,
        syn::Body::Struct(syn::VariantData::Unit) => Vec::new(),
        syn::Body::Enum(..) => panic!("Message can not be derived for an enum"),
    };

    let fields = fields.into_iter().filter_map(Field::extract);

    let dummy_const = syn::Ident::new(format!("_IMPL_SERIALIZE_FOR_{}", ident));

    let expanded = quote! {
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const #dummy_const: () = {
            extern crate proto;
            use std::any::Any;
            use std::any::TypeId;
            use std::io::Read;
            use std::io::Result;
            use std::io::Write;

            #[automatically_derived]
            impl proto::Field for Option<#ident> {
                fn merge_from<R>(&mut self, r: &mut R) -> Result<()> where R: Read {

                }
                fn write_to<W>(&self, w: &mut W) -> Result<()> where W: Write {
                }
                fn wire_type() -> proto::WireType {
                    proto::WireType::LengthDelimited;
                }
                fn wire_len(&self) -> usize {
                }

            }

            #[automatically_derived]
            impl proto::Message for #ident {

                fn write_to(&self, w: &mut Write) -> Result<()> {
                    unimplemented!()
                }

                fn write_length_delimited_to(&self, w: &mut Write) -> Result<()> {
                    unimplemented!()
                }


                fn merge_from(&mut self, r: &mut Read) -> Result<()> {
                    unimplemented!()
                }

                fn merge_delimited_from(&mut self, r: &mut Read) -> Result<()> {
                    unimplemented!()
                }

                fn type_id(&self) -> TypeId {
                    TypeId::of::<#ident>()
                }

                fn as_any(&self) -> &Any {
                    self
                }

                fn as_any_mut(&mut self) -> &mut Any {
                    self
                }

                fn into_any(self: Box<Self>) -> Box<Any> {
                    self
                }

            }
        };
    };

    expanded.parse().unwrap()
}

#[proc_macro_derive(Enumeration)]
pub fn enumeration(input: TokenStream) -> TokenStream {
    let source = input.to_string();
    let ast = syn::parse_derive_input(&source).expect("unable to parse enumeration token stream");

    // Build the output
    //let expanded = expand_num_fields(&ast);

    // Return the generated impl as a TokenStream
    //expanded.parse().unwrap()
    unimplemented!()
}

#[proc_macro_derive(Oneof)]
pub fn oneof(input: TokenStream) -> TokenStream {
    let source = input.to_string();
    let ast = syn::parse_derive_input(&source).expect("unable to parse oneof token stream");

    // Build the output
    //let expanded = expand_num_fields(&ast);

    // Return the generated impl as a TokenStream
    //expanded.parse().unwrap()
    unimplemented!()
}
