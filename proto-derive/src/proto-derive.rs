// The `quote!` macro requires deep recursion.
#![recursion_limit = "192"]

extern crate proc_macro;
extern crate proto;
extern crate syn;


#[macro_use]
extern crate quote;

use std::mem;

use proc_macro::TokenStream;

enum Field {
    Included {
        field: syn::Field,
        tag: u32,
        fixed: bool,
        packed: bool,
        signed: bool,
    },
    Ignored {
        field: syn::Field,
    },
}

impl Field {
    fn field(&self) -> &syn::Field {
        match *self {
            Field::Included { ref field, .. } => field,
            Field::Ignored { ref field, .. } => field,
        }
    }

    fn ignored(&self) -> bool {
        match *self {
            Field::Included { .. } => false,
            Field::Ignored { .. } => true,
        }
    }
}

impl From<syn::Field> for Field {
    fn from(mut field: syn::Field) -> Field {
        let mut proto_items = Vec::new();

        for attr in mem::replace(&mut field.attrs, Vec::new()) {
            match attr.value {
                syn::MetaItem::List(ref ident, ref items) if ident == "proto" => proto_items.extend(items.iter().cloned()),
                _ => field.attrs.push(attr),
            }
        }

        let mut tag = None;
        let mut fixed = false;
        let mut packed = false;
        let mut signed = false;
        let mut ignore = false;

        for meta_item in proto_items {
            match meta_item {
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

                // Handle `#[proto(packed)]` and `#[proto(packed = false)].
                syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref name)) if name == "packed" => packed = true,
                syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, syn::Lit::Bool(value))) if name == "packed" => packed = value,

                // Handle `#[proto(signed)]` and `#[proto(signed = false)]`.
                syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref name)) if name == "signed" => signed = true,
                syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, syn::Lit::Bool(value))) if name == "signed" => signed = value,

                // Handle `#[proto(ignore)]` and `#[proto(ignore = false)]`.
                syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref name)) if name == "ignore" => ignore = true,
                syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, syn::Lit::Bool(value))) if name == "ignore" => ignore = value,

                syn::NestedMetaItem::MetaItem(ref meta_item) => panic!("unknown proto field attribute `{}`", meta_item.name()),
                syn::NestedMetaItem::Literal(_) => panic!("unexpected literal in serde field attribute"),
            }
        }

        match tag {
            Some(tag) => {
                if ignore {
                    panic!("ignored proto field must not have a tag attribute");
                }

                if tag < 1 {
                    panic!("proto tag must not be zero");
                } else if tag > (1 << 29) - 1 {
                    panic!("proto tag must be below 2^29 - 1");
                }

                Field::Included {
                    field: field,
                    tag: tag as u32,
                    fixed: fixed,
                    packed: packed,
                    signed: signed,
                }
            },
            None => {
                if !ignore {
                    panic!("proto field must have a tag attribute");
                }
                Field::Ignored {
                    field: field,
                }
            }
        }
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

    let fields = fields.into_iter().map(Field::from).collect::<Vec<_>>();

    let dummy_const = syn::Ident::new(format!("_IMPL_SERIALIZE_FOR_{}", ident));

    let expanded = quote! {
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const #dummy_const: () = {
            extern crate proto as _proto;
            use std::any::Any;
            use std::any::TypeId;
            use std::io::Read;
            use std::io::Result;
            use std::io::Write;
            #[automatically_derived]
            impl _proto::Message for #ident {

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
