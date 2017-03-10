// The `quote!` macro requires deep recursion.
#![recursion_limit = "1024"]

extern crate proc_macro;
//extern crate proto;
extern crate syn;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;

use quote::Tokens;

fn concat_tokens(mut sum: Tokens, rest: Tokens) -> Tokens {
    sum.append(rest.as_str());
    sum
}

struct Field {
    ident: syn::Ident,
    kind: quote::Tokens,
    default: Option<syn::Lit>,
    tag: u32,
}

impl Field {
    fn extract(field: syn::Field) -> Option<Field> {
        let mut tag = None;
        let mut default = None;
        let mut fixed = false;
        let mut signed = false;
        let mut ignore = false;

        let mut fixed_key = false;
        let mut signed_key = false;
        let mut fixed_value = false;
        let mut signed_value = false;

        let attrs = field.attrs;
        let ident = field.ident.expect("Message struct has unnamed field");

        {
            // Get the metadata items belonging to 'proto' list attributes (e.g. #[proto(foo, bar="baz")]).
            let proto_items = attrs.iter().flat_map(|attr| {
                match attr.value {
                    syn::MetaItem::List(ref ident, ref items) if ident == "proto" => items.into_iter(),
                    _ => [].into_iter(),
                }
            });

            for item in proto_items {
                match *item {
                    // Handle `#[proto(tag = 1)] and #[proto(tag = "1")]`.
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

                    // Handle `#[proto(fixed_key)]` and `#[proto(fixed_key = false)].
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref name)) if name == "fixed_key" => fixed_key = true,
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, syn::Lit::Bool(value))) if name == "fixed_key" => fixed_key = value,

                    // Handle `#[proto(signed_key)]` and `#[proto(signed_key = false)]`.
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref name)) if name == "signed_key" => signed_key = true,
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, syn::Lit::Bool(value))) if name == "signed_key" => signed_key = value,

                    // Handle `#[proto(fixed_value)]` and `#[proto(fixed_value = false)].
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref name)) if name == "fixed_value" => fixed_value = true,
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, syn::Lit::Bool(value))) if name == "fixed_value" => fixed_key = value,

                    // Handle `#[proto(signed_value)]` and `#[proto(signed_value = false)]`.
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref name)) if name == "signed_value" => signed_value = true,
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, syn::Lit::Bool(value))) if name == "signed_value" => signed_value = value,

                    // Handle `#[proto(ignore)]` and `#[proto(ignore = false)]`.
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref name)) if name == "ignore" => ignore = true,
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, syn::Lit::Bool(value))) if name == "ignore" => ignore = value,

                    // Handle `#[proto(default = "")]`
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, ref value)) if name == "default" => default = Some(value.clone()),

                    syn::NestedMetaItem::MetaItem(ref meta_item) => panic!("unknown proto field attribute item `{}`", meta_item.name()),
                    syn::NestedMetaItem::Literal(_) => panic!("unexpected literal in serde field attribute"),
                }
            }
        }

        let (tag, kind) = match (tag, fixed, signed, fixed_key, signed_key, fixed_value, signed_value, ignore) {
            (None, false, false, false, false, false, false, true) => return None,

            (Some(tag), _, _, _, _, _, _, _) if tag >= (1 << 29) as u64 => panic!("proto tag must be less than 2^29"),
            (Some(tag), _, _, _, _, _, _, _) if tag < 1 as u64          => panic!("proto tag must be greater than 1"),

            (Some(_), _, _, _, _, _, _, true) => panic!("ignored proto field must not have a tag attribute"),
            (None, _, _, _, _, _, _, false)   => panic!("proto field must have a tag attribute"),

            (Some(tag), false, false, false, false, false, false, _) => (tag as u32, quote!(field::Default)),
            (Some(tag), true, false, false, false, false, false, _)  => (tag as u32, quote!(field::Fixed)),
            (Some(tag), false, true, false, false, false, false, _)  => (tag as u32, quote!(field::Signed)),

            (Some(tag), false, false, true, false, false, false, _) => (tag as u32, quote!((field::Fixed, field::Default))),
            (Some(tag), false, false, false, true, false, false, _) => (tag as u32, quote!((field::Signed, field::Default))),
            (Some(tag), false, false, false, false, true, false, _) => (tag as u32, quote!((field::Default, field::Fixed))),
            (Some(tag), false, false, false, false, false, true, _) => (tag as u32, quote!((field::Default, field::Signed))),

            (Some(tag), false, false, true, false, true, false, _) => (tag as u32, quote!((field::Fixed, field::Fixed))),
            (Some(tag), false, false, true, false, false, true, _) => (tag as u32, quote!((field::Fixed, field::Signed))),
            (Some(tag), false, false, false, true, true, false, _) => (tag as u32, quote!((field::Signed, field::Fixed))),
            (Some(tag), false, false, false, true, false, true, _) => (tag as u32, quote!((field::Signed, field::Signed))),

            (None, true, _, _, _, _, _, _)  => panic!("ignored proto field must not be fixed"),
            (None, _, true, _, _, _, _, _)  => panic!("ignored proto field must not be signed"),
            (None, _, _, true, _, _, _, _)  => panic!("ignored proto field must not be fixed_key"),
            (None, _, _, _, true, _, _, _)  => panic!("ignored proto field must not be signed_key"),
            (None, _, _, _, _, true, _, _)  => panic!("ignored proto field must not be fixed_value"),
            (None, _, _, _, _, _, true, _)  => panic!("ignored proto field must not be signed_value"),


            (_, true, true, _, _, _, _, _) => panic!("proto field must not be fixed and signed"),
            (_, true, _, true, _, _, _, _) => panic!("proto field must not be fixed and fixed_key"),
            (_, true, _, _, true, _, _, _) => panic!("proto field must not be fixed and signed_key"),
            (_, true, _, _, _, true, _, _) => panic!("proto field must not be fixed and fixed_value"),
            (_, true, _, _, _, _, true, _) => panic!("proto field must not be fixed and signed_value"),
            (_, _, true, true, _, _, _, _) => panic!("proto field must not be signed and fixed_key"),
            (_, _, true, _, true, _, _, _) => panic!("proto field must not be signed and signed_key"),
            (_, _, true, _, _, true, _, _) => panic!("proto field must not be signed and fixed_value"),
            (_, _, true, _, _, _, true, _) => panic!("proto field must not be signed and signed_value"),
            (_, _, _, true, true, _, _, _) => panic!("proto field must not be fixed_key and signed_key"),
            (_, _, _, _, _, true, true, _) => panic!("proto field must not be fixed_value and signed_value"),
        };

        Some(Field {
            ident: ident,
            kind: kind,
            default: default,
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

    let fields = fields.into_iter().filter_map(Field::extract).collect::<Vec<_>>();

    let mut tags = fields.iter().map(|field| field.tag).collect::<Vec<_>>();
    tags.sort();
    tags.dedup();
    if tags.len() != fields.len() {
        panic!("Message '{}' has fields with duplicate tags", ident);
    }


    let dummy_const = syn::Ident::new(format!("_IMPL_MESSAGE_FOR_{}", ident));
    let wire_len = wire_len(&fields);

    let write_to = fields.iter().map(|field| {
        let kind = &field.kind;
        let tag = field.tag;
        let field = &field.ident;
        quote! {
            Field::<#kind>::write_to(&self.#field, #tag, w)
                           .map_err(|error| {
                               Error::new(error.kind(),
                                           format!(concat!("failed to write field ", stringify!(#ident),
                                                           ".", stringify!(#field), ": {}"),
                                                   error))
                           })?;
        }
    }).fold(Tokens::new(), concat_tokens);

    let merge_from = fields.iter().map(|field| {
        let tag = field.tag;
        let kind = &field.kind;
        let field = &field.ident;
        quote!{ #tag => Field::<#kind>::merge_from(&mut self.#field, wire_type, r, &mut limit)
                              .map_err(|error| {
                                  Error::new(error.kind(),
                                             format!(concat!("failed to read field ", stringify!(#ident),
                                                             ".", stringify!(#field), ": {}"),
                                                     error))
                              })?, }
    }).fold(Tokens::new(), concat_tokens);

    let default = default(&fields);

    let expanded = quote! {
        #[allow(
            non_upper_case_globals,
            unused_attributes,
            unused_imports,
            unused_qualifications,
            unused_variables
        )]
        const #dummy_const: () = {
            extern crate proto;
            use std::any::{Any, TypeId};
            use std::io::{
                Error,
                Read,
                Result,
                Write,
            };
            use proto::field::{self, Field};

            #[automatically_derived]
            impl proto::Message for #ident {
                fn write_to(&self, w: &mut Write) -> Result<()> {
                    #write_to
                    Ok(())
                }

                fn merge_from(&mut self, len: usize, r: &mut Read) -> Result<()> {
                    let mut limit = len;
                    while limit > 0 {
                        let (wire_type, tag) = field::read_key_from(r, &mut limit)?;
                        match tag {
                            #merge_from
                            _ => field::skip_field(wire_type, r, &mut limit)?,
                        }
                    }
                    Ok(())
                }

                fn wire_len(&self) -> usize {
                    #wire_len
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

            #[automatically_derived]
            impl Default for #ident {
                fn default() -> #ident {
                    #ident {
                        #default
                    }
                }
            }
        };
    };

    expanded.parse().unwrap()
}

fn wire_len(fields: &[Field]) -> Tokens {
    fields.iter().map(|field| {
        let kind = &field.kind;
        let ident = &field.ident;
        let tag = field.tag;
        quote!(Field::<#kind>::wire_len(&self.#ident, #tag))
    })
    .fold(quote!(0), |mut sum, expr| {
        sum.append("+");
        sum.append(expr.as_str());
        sum
    })
}

fn default(fields: &[Field]) -> Tokens {
    fields.iter().map(|field| {
        let ident = &field.ident;
        match field.default {
            Some(ref default) => quote!(#ident: #default.parse().unwrap(),),
            None => quote!(#ident: Default::default(),),
        }
    })
    .fold(Tokens::new(), concat_tokens)
}

#[proc_macro_derive(Enumeration, attributes(proto))]
pub fn enumeration(input: TokenStream) -> TokenStream {
    let syn::DeriveInput { ident, generics, attrs, body, .. } =
        syn::parse_derive_input(&input.to_string()).expect("unable to parse message type");

    if !generics.lifetimes.is_empty() ||
       !generics.ty_params.is_empty() ||
       !generics.where_clause.predicates.is_empty() {
        panic!("Enumeration may not be derived for generic type");
    }

    let variants = match body {
        syn::Body::Struct(..) => panic!("Enumeration can not be derived for a struct"),
        syn::Body::Enum(variants) => variants,
    };

    let variants = variants.into_iter().map(|syn::Variant { ident: variant, data, discriminant, .. }| {
        if let syn::VariantData::Unit = data {
            if let Some(discriminant) = discriminant {
                (variant, discriminant)
            } else {
                panic!("Enumeration variants must have a discriminant value: {}::{}", ident, variant);
            }
        } else {
            panic!("Enumeration variants may not have fields: {}::{}", ident, variant);
        }
    }).collect::<Vec<_>>();

    if variants.is_empty() {
        panic!("Enumeration must have at least one variant: {}", ident);
    }

    let repr = attrs.into_iter()
                    .filter_map(|mut attr: syn::Attribute| match attr.value {
                        syn::MetaItem::List(ref attr, ref mut repr) if attr == "repr" => repr.pop(),
                        _ => None,
                    })
                    .last()
                    .map_or_else(|| quote!(i32), |repr| quote!(#repr));

    let default = variants[0].0.clone();

    let dummy_const = syn::Ident::new(format!("_IMPL_ENUMERATION_FOR_{}", ident));
    let is_valid = variants.iter()
                           .map(|&(_, ref value)| quote!(#value => true,))
                           .fold(Tokens::new(), concat_tokens);
    let from = variants.iter()
                       .map(|&(ref variant, ref value)| quote!(#value => #ident::#variant,))
                       .fold(Tokens::new(), concat_tokens);
    let from_str = variants.iter()
                           .map(|&(ref variant, _)| quote!(stringify!(#variant) => Ok(#ident::#variant),))
                           .fold(Tokens::new(), concat_tokens);

    let expanded = quote! {
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const #dummy_const: () = {
            extern crate proto;
            use std::io::{
                Read,
                Result,
                Write,
            };
            use std::result;
            use std::str::FromStr;

            use proto::field::{
                Field,
                self,
                WireType,
            };

            impl #ident {
                fn is_valid(value: #repr) -> bool {
                    match value {
                        #is_valid
                        _ => false,
                    }
                }
            }

            #[automatically_derived]
            impl Field for #ident {
                fn write_to(&self, tag: u32, w: &mut Write) -> Result<()> {
                    Field::<field::Default>::write_to(&(*self as i32), tag, w)
                }

                fn merge_from(&mut self, wire_type: WireType, r: &mut Read, limit: &mut usize) -> Result<()> {
                    let mut value: i32 = 0;
                    Field::<field::Default>::merge_from(&mut value, wire_type, r, limit)?;
                    *self = #ident::from(value as #repr);
                    Ok(())
                }

                fn wire_len(&self, tag: u32) -> usize {
                    Field::<field::Default>::wire_len(&(*self as i32), tag)
                }
            }

            #[automatically_derived]
            impl Default for #ident {
                fn default() -> #ident {
                    #ident::#default
                }
            }

            #[automatically_derived]
            impl From<#repr> for #ident {
                fn from(value: #repr) -> #ident {
                    match value {
                        #from
                        _ => #ident::#default,
                    }
                }
            }

            #[automatically_derived]
            impl FromStr for #ident {
                type Err = String;
                fn from_str(s: &str) -> result::Result<#ident, String> {
                    match s {
                        #from_str
                        _ => Err(format!(concat!("unknown ", stringify!(#ident), " variant: {}"), s)),
                    }
                }
            }
        };
    };

    expanded.parse().unwrap()
}

#[proc_macro_derive(Oneof, attributes(proto))]
pub fn oneof(input: TokenStream) -> TokenStream {
    //let source = input.to_string();
    //let ast = syn::parse_derive_input(&source).expect("unable to parse oneof token stream");



    // Build the output
    //let expanded = expand_num_fields(&ast);

    // Return the generated impl as a TokenStream
    //expanded.parse().unwrap()
    unimplemented!()
}
