// The `quote!` macro requires deep recursion.
#![recursion_limit = "1024"]

extern crate itertools;
extern crate proc_macro;
extern crate syn;

#[macro_use]
extern crate quote;

use itertools::Itertools;
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
    tags: Vec<u32>,
}

impl Field {
    fn extract(field: syn::Field) -> Option<Field> {
        let mut tags = Vec::new();
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
                    // Handle `#[proto(tag = 1)]` and `#[proto(tag = "1")]`.
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, syn::Lit::Int(value, _))) if name == "tag" => tags.push(value),
                    syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, syn::Lit::Str(ref value, _))) if name == "tag" => {
                        match value.parse() {
                            Ok(value) => tags.push(value),
                            Err(..) => panic!("tag attribute value must be an integer"),
                        }
                    },

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

                    syn::NestedMetaItem::MetaItem(ref meta_item) => panic!("unknown proto field attribute item: {:?}", meta_item),
                    syn::NestedMetaItem::Literal(ref literal) => panic!("unexpected literal in proto field attribute: `{:?}`", literal),
                }
            }
        }

        if !tags.iter().all(|&tag| tag > 0) {
            panic!("proto tag must be greater than 0");
        }
        if !tags.iter().all(|&tag| tag < (1 << 29)) {
            panic!("proto tag must be less than 2^29");
        }

        let tags = tags.into_iter().map(|tag| tag as u32).collect::<Vec<_>>();

        let kind = match (!tags.is_empty(), fixed, signed, fixed_key, signed_key, fixed_value, signed_value, ignore) {
            (false, false, false, false, false, false, false, true) => return None,

            (true, _, _, _, _, _, _, true) => panic!("ignored proto field must not have a tag attribute"),
            (false, _, _, _, _, _, _, false)   => panic!("proto field must have a tag attribute"),

            (true, false, false, false, false, false, false, _) => quote!(field::Default),
            (true, true, false, false, false, false, false, _)  => quote!(field::Fixed),
            (true, false, true, false, false, false, false, _)  => quote!(field::Signed),

            (true, false, false, true, false, false, false, _) => quote!((field::Fixed, field::Default)),
            (true, false, false, false, true, false, false, _) => quote!((field::Signed, field::Default)),
            (true, false, false, false, false, true, false, _) => quote!((field::Default, field::Fixed)),
            (true, false, false, false, false, false, true, _) => quote!((field::Default, field::Signed)),

            (true, false, false, true, false, true, false, _) => quote!((field::Fixed, field::Fixed)),
            (true, false, false, true, false, false, true, _) => quote!((field::Fixed, field::Signed)),
            (true, false, false, false, true, true, false, _) => quote!((field::Signed, field::Fixed)),
            (true, false, false, false, true, false, true, _) => quote!((field::Signed, field::Signed)),

            (false, true, _, _, _, _, _, _)  => panic!("ignored proto field must not be fixed"),
            (false, _, true, _, _, _, _, _)  => panic!("ignored proto field must not be signed"),
            (false, _, _, true, _, _, _, _)  => panic!("ignored proto field must not be fixed_key"),
            (false, _, _, _, true, _, _, _)  => panic!("ignored proto field must not be signed_key"),
            (false, _, _, _, _, true, _, _)  => panic!("ignored proto field must not be fixed_value"),
            (false, _, _, _, _, _, true, _)  => panic!("ignored proto field must not be signed_value"),

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
            tags: tags,
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

    let mut tags = fields.iter().flat_map(|field| &field.tags).collect::<Vec<_>>();
    let num_tags = tags.len();
    tags.sort();
    tags.dedup();
    if tags.len() != num_tags {
        panic!("Message '{}' has fields with duplicate tags", ident);
    }

    let dummy_const = syn::Ident::new(format!("_IMPL_MESSAGE_FOR_{}", ident));
    let wire_len = wire_len(&fields);

    let write_to = fields.iter().map(|field| {
        let kind = &field.kind;
        let tag = field.tags[0];
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
        let tags = field.tags.iter().map(|tag| quote!(#tag)).intersperse(quote!(|)).fold(Tokens::new(), concat_tokens);
        let kind = &field.kind;
        let field = &field.ident;
        quote!{ #tags => Field::<#kind>::merge_from(&mut self.#field, tag, wire_type, r, &mut limit)
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
                Write,
            };
            use proto::field::{self, Field, WireType};

            #[automatically_derived]
            impl proto::Message for #ident {
                fn write_to(&self, w: &mut Write) -> ::std::io::Result<()> {
                    #write_to
                    Ok(())
                }

                fn merge_from(&mut self, len: usize, r: &mut Read) -> ::std::io::Result<()> {
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
        let tag = field.tags[0];
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
                    .map_or_else(|| quote!(i64), |repr| quote!(#repr));

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
                Write,
            };
            use std::str::FromStr;

            use proto::field::{
                Field,
                WireType,
                self,
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
                fn write_to(&self, tag: u32, w: &mut Write) -> ::std::io::Result<()> {
                    Field::<field::Default>::write_to(&(*self as i64), tag, w)
                }

                fn read_from(tag: u32, wire_type: WireType, r: &mut Read, limit: &mut usize) -> ::std::io::Result<#ident> {
                    <i64 as Field<field::Default>>::read_from(tag, wire_type, r, limit).map(From::from)
                }

                fn wire_len(&self, tag: u32) -> usize {
                    Field::<field::Default>::wire_len(&(*self as i64), tag)
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
                fn from_str(s: &str) -> ::std::result::Result<#ident, String> {
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
    let syn::DeriveInput { ident, generics, body, .. } =
        syn::parse_derive_input(&input.to_string()).expect("unable to parse message type");

    if !generics.lifetimes.is_empty() ||
       !generics.ty_params.is_empty() ||
       !generics.where_clause.predicates.is_empty() {
        panic!("Oneof may not be derived for generic type");
    }

    let variants = match body {
        syn::Body::Enum(variants) => variants,
        syn::Body::Struct(..) => panic!("Oneof can not be derived for a struct"),
    };

    // Map the variants into 'fields'.
    let fields = variants.into_iter().map(|variant| {
        let ident = variant.ident;
        let attrs = variant.attrs;
        if let syn::VariantData::Tuple(mut fields) = variant.data {
            if fields.len() != 1 {
                panic!("Oneof enum must contain only tuple variants with a single field");
            }
            let field = fields.pop().unwrap();
            Field::extract(syn::Field {
                ident: Some(ident),
                vis: field.vis,
                attrs: attrs,
                ty: field.ty,
            }).expect("Oneof fields may not be ignored")
        } else {
            panic!("Oneof enum must contain only tuple variants with a single field");
        }
    }).collect::<Vec<_>>();

    let mut tags = fields.iter().flat_map(|field| {
        if field.tags.len() > 1 {
            panic!("proto oneof variants may only have a single tag: {}.{}",
                   ident, field.ident);
        }
        &field.tags
    }).collect::<Vec<_>>();
    tags.sort();
    tags.dedup();
    if tags.len() != fields.len() {
        panic!("proto oneof variants may not have duplicate tags: {}", ident);
    }

    let dummy_const = syn::Ident::new(format!("_IMPL_ONEOF_FOR_{}", ident));

    let write_to = fields.iter().map(|field| {
        let kind = &field.kind;
        let name = &field.ident;
        let tag = field.tags[0];
        quote! { #ident::#name(ref value) => Field::<#kind>::write_to(value, #tag, w), }
    }).fold(Tokens::new(), concat_tokens);

    let read_from = fields.iter().map(|field| {
        let kind = &field.kind;
        let name = &field.ident;
        let tag = field.tags[0];
        quote! { #tag => Field::<#kind>::read_from(tag, wire_type, r, limit).map(|value| #ident::#name(value)), }
    }).fold(Tokens::new(), concat_tokens);

    let wire_len = fields.iter().map(|field| {
        let kind = &field.kind;
        let name = &field.ident;
        let tag = field.tags[0];
        quote! { #ident::#name(ref value) => Field::<#kind>::wire_len(value, #tag), }
    }).fold(Tokens::new(), concat_tokens);

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
                Write,
            };
            use proto::field::{self, Field, WireType};

            #[automatically_derived]
            impl proto::field::Field for #ident {
                fn write_to(&self, _tag: u32, w: &mut Write) -> ::std::io::Result<()> {
                    match *self {
                        #write_to
                    }
                }

                fn read_from(tag: u32, wire_type: WireType, r: &mut Read, limit: &mut usize) -> ::std::io::Result<#ident> {
                    match tag {
                        #read_from
                        // TODO: test coverage of this case
                        _ => panic!("proto oneof tag misconfiguration: missing variant of {} with tag: {}",
                                    stringify!(#ident), tag),
                    }
                }

                fn wire_len(&self, tag: u32) -> usize {
                    match *self {
                        #wire_len
                    }
                }
            }
        };
    };

    expanded.parse().unwrap()
}
