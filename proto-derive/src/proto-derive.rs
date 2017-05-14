// The `quote!` macro requires deep recursion.
#![recursion_limit = "1024"]

extern crate itertools;
extern crate proc_macro;
extern crate syn;

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate quote;

use std::str;

use itertools::Itertools;
use proc_macro::TokenStream;
use quote::Tokens;

// Proc-macro crates can't export anything, so error chain definitions go in a private module.
mod error {
    error_chain!();
}
use error::*;

mod field;
use field::Field;

fn concat_tokens(mut sum: Tokens, rest: Tokens) -> Tokens {
    sum.append(rest.as_str());
    sum
}

fn try_message(input: TokenStream) -> Result<TokenStream> {
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

    let fields = fields.into_iter()
                       .enumerate()
                       .flat_map(|(idx, field)| {
                           let field_ident = field.ident
                                                   .unwrap_or_else(|| syn::Ident::new(idx.to_string()));
                           match Field::new(field_ident.clone(), &field.attrs) {
                               Ok(Some(field)) => Some(Ok(field)),
                               Ok(None) => None,
                               Err(err) => Some(Err(err).chain_err(|| {
                                   format!("invalid message field {}.{}",
                                           ident, field_ident)
                               })),
                           }
                       })
                       .collect::<Result<Vec<Field>>>()?;

    let mut tags = fields.iter().flat_map(|field| field.tags()).collect::<Vec<_>>();
    let num_tags = tags.len();
    tags.sort();
    tags.dedup();
    if tags.len() != num_tags {
        bail!("message {} has fields with duplicate tags", ident);
    }

    let dummy_const = syn::Ident::new(format!("_IMPL_MESSAGE_FOR_{}", ident));

    let encoded_len = fields.iter()
                            .map(Field::encoded_len)
                            .fold(quote!(0), |mut sum, expr| {
                                sum.append("+");
                                sum.append(expr.as_str());
                                sum
                            });

    let encode = fields.iter().map(Field::encode).fold(Tokens::new(), concat_tokens);

    let merge = fields.iter().map(|field| {
        let merge = field.merge(&syn::Ident::new("tag"), &syn::Ident::new("wire_type"));
        let tags = field.tags().iter().map(|tag| quote!(#tag)).intersperse(quote!(|)).fold(Tokens::new(), concat_tokens);
        let field_ident = field.ident();
        quote! { #tags => #merge.map_err(|error| {
            ::std::io::Error::new(
                error.kind(),
                format!(concat!("failed to decode field ", stringify!(#ident), ".", stringify!(#field_ident), ": {}"),
                        error))
        })?, }
    }).fold(Tokens::new(), concat_tokens);

    let default = fields.iter()
                        .map(|field| {
                            let ident = field.ident();
                            let value = field.default();
                            quote!(#ident: #value,)
                        })
                        .fold(Tokens::new(), concat_tokens);

    let expanded = quote! {
        #[allow(
            non_upper_case_globals,
            unused_attributes,
            unused_imports,
            unused_qualifications,
            unused_variables
        )]
        const #dummy_const: () = {

            extern crate proto as _proto;
            extern crate bytes as _bytes;

            #[automatically_derived]
            impl _proto::Message for #ident {
                #[inline]
                fn encode_raw<B>(&self, buf: &mut B) where B: _bytes::BufMut {
                    #encode
                }

                #[inline]
                fn merge<B>(&mut self, buf: &mut _bytes::Take<B>) -> ::std::io::Result<()> where B: _bytes::Buf {
                    while _bytes::Buf::has_remaining(buf) {
                        let (tag, wire_type) = _proto::encoding::decode_key(buf)?;
                        match tag {
                            #merge
                            _ => _proto::encoding::skip_field(wire_type, buf)?,
                        }
                    }
                    Ok(())
                }

                #[inline]
                fn encoded_len(&self) -> usize {
                    #encoded_len
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

    expanded.parse::<TokenStream>().map_err(|err| Error::from(format!("{:?}", err)))
}

#[proc_macro_derive(Message, attributes(proto))]
pub fn message(input: TokenStream) -> TokenStream {
    try_message(input).unwrap()
}

/*
#[proc_macro_derive(Enumeration, attributes(proto))]
pub fn enumeration(input: TokenStream) -> TokenStream {
    let syn::DeriveInput { ident, generics, attrs, body, .. } =
        syn::parse_derive_input(&input.to_string()).expect("unable to parse enumeration type");

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
            extern crate bytes as _bytes;
            extern crate proto as _proto;

            impl #ident {
                fn is_valid(value: #repr) -> bool {
                    match value {
                        #is_valid
                        _ => false,
                    }
                }
            }

            #[automatically_derived]
            impl _proto::field::Field<_proto::field::Enumeration> for #ident {

                #[inline]
                fn encode<B>(&self, tag: u32, buf: &mut B) where B: _bytes::BufMut {
                    _proto::encoding::encode_key(tag, _proto::encoding::WireType::Varint, buf);
                    _proto::encoding::encode_varint(*self as u64, buf);
                }

                #[inline]
                fn merge<B>(&mut self, _tag: u32, wire_type: _proto::encoding::WireType, buf: &mut _bytes::Take<B>) -> ::std::io::Result<()> where B: _bytes::Buf {
                    _proto::encoding::check_wire_type(_proto::encoding::WireType::Varint, wire_type)?;
                    *self = #ident::from(_proto::encoding::decode_varint(buf)? as #repr);
                    Ok(())
                }

                #[inline]
                fn encoded_len(&self, tag: u32) -> usize {
                    _proto::encoding::key_len(tag) + _proto::encoding::encoded_len_varint(*self as u64)
                }
            }

            impl _proto::field::Type<_proto::field::Enumeration> for #ident {}

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
            impl ::std::str::FromStr for #ident {
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

    let encode = fields.iter().map(|field| {
        let kind = &field.kind;
        let name = &field.ident;
        let tag = field.tags[0];
        quote! { #ident::#name(ref value) => _proto::field::Field::<#kind>::encode(value, #tag, buf), }
    }).fold(Tokens::new(), concat_tokens);

    let merge = fields.iter().map(|field| {
        let kind = &field.kind;
        let name = &field.ident;
        let tag = field.tags[0];
        quote! { #tag => {
            let mut value = ::std::default::Default::default();
            _proto::field::Field::<#kind>::merge(&mut value, tag, wire_type, buf)?;
            #ident::#name(value)
        },
        }
    }).fold(Tokens::new(), concat_tokens);

    let encoded_len = fields.iter().map(|field| {
        let kind = &field.kind;
        let name = &field.ident;
        let tag = field.tags[0];
        quote! { #ident::#name(ref value) => _proto::field::Field::<#kind>::encoded_len(value, #tag), }
    }).fold(Tokens::new(), concat_tokens);

    let default = {
        let field_ident = &fields[0].ident;
        quote!(#ident::#field_ident(::std::default::Default::default()))
    };

    let expanded = quote! {
        #[allow(
            non_upper_case_globals,
            unused_attributes,
            unused_imports,
            unused_qualifications,
            unused_variables
        )]
        const #dummy_const: () = {
            extern crate bytes as _bytes;
            extern crate proto as _proto;

            #[automatically_derived]
            impl _proto::field::Field<_proto::field::Oneof> for #ident {
                #[inline]
                fn encode<B>(&self, tag: u32, buf: &mut B) where B: _bytes::BufMut {
                    match *self {
                        #encode
                    }
                }

                #[inline]
                fn merge<B>(&mut self, tag: u32, wire_type: _proto::encoding::WireType, buf: &mut _bytes::Take<B>) -> ::std::io::Result<()> where B: _bytes::Buf {
                    *self = match tag {
                        #merge
                        _ => panic!("unknown tag: {}", tag),
                    };
                    ::std::result::Result::Ok(())
                }

                #[inline]
                fn encoded_len(&self, tag: u32) -> usize {
                    match *self {
                        #encoded_len
                    }
                }
            }

            impl ::std::default::Default for #ident {
                fn default() -> #ident {
                    #default
                }
            }

            impl _proto::field::Type<_proto::field::Oneof> for #ident {}
        };
    };

    expanded.parse().unwrap()
}
*/
