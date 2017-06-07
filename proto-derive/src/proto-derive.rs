// The `quote!` macro requires deep recursion.
#![recursion_limit = "4096"]

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
use syn::Ident;

// Proc-macro crates can't export anything, so error chain definitions go in a private module.
mod error {
    error_chain!();
}
use error::*;

mod field;
use field::Field;

fn try_message(input: TokenStream) -> Result<TokenStream> {
    let syn::DeriveInput { ident, generics, body, .. } = syn::parse_derive_input(&input.to_string())?;

    if !generics.lifetimes.is_empty() ||
       !generics.ty_params.is_empty() ||
       !generics.where_clause.predicates.is_empty() {
        bail!("Message may not be derived for generic type");
    }

    let fields = match body {
        syn::Body::Struct(syn::VariantData::Struct(fields)) => fields,
        syn::Body::Struct(syn::VariantData::Tuple(fields)) => fields,
        syn::Body::Struct(syn::VariantData::Unit) => Vec::new(),
        syn::Body::Enum(..) => bail!("Message can not be derived for an enum"),
    };

    let fields = fields.into_iter()
                       .enumerate()
                       .flat_map(|(idx, field)| {
                           let field_ident = field.ident
                                                  .unwrap_or_else(|| Ident::new(idx.to_string()));
                           match Field::new(field.attrs) {
                               Ok(Some(field)) => Some(Ok((field_ident, field))),
                               Ok(None) => None,
                               Err(err) => Some(Err(err).chain_err(|| {
                                   format!("invalid message field {}.{}",
                                           ident, field_ident)
                               })),
                           }
                       })
                       .collect::<Result<Vec<(Ident, Field)>>>()?;

    let mut tags = fields.iter().flat_map(|&(_, ref field)| field.tags()).collect::<Vec<_>>();
    let num_tags = tags.len();
    tags.sort();
    tags.dedup();
    if tags.len() != num_tags {
        bail!("message {} has fields with duplicate tags", ident);
    }

    let dummy_const = Ident::new(format!("_IMPL_MESSAGE_FOR_{}", ident));

    let encoded_len = fields.iter()
                            .map(|&(ref field_ident, ref field)| {
                                field.encoded_len(&Ident::new(format!("self.{}", field_ident)))
                            });

    let encode = fields.iter()
                       .map(|&(ref field_ident, ref field)| {
                           field.encode(&Ident::new(format!("self.{}", field_ident)))
                       });

    let merge = fields.iter().map(|&(ref field_ident, ref field)| {
        let merge = field.merge(&Ident::new(format!("self.{}", field_ident)));
        let tags = field.tags().into_iter().map(|tag| quote!(#tag)).intersperse(quote!(|));
        quote!(#(#tags)* => #merge.map_err(|error| map_err(stringify!(#field_ident), error))?,)
    });

    let default = fields.iter()
                        .map(|&(ref field_ident, ref field)| {
                            let value = field.default();
                            quote!(#field_ident: #value,)
                        });

    let methods = fields.iter()
                        .flat_map(|&(ref field_ident, ref field)| field.methods(field_ident))
                        .collect::<Vec<_>>();
    let methods = if methods.is_empty() {
        quote!()
    } else {
        quote! {
            impl #ident {
                #(#methods)*
            }
        }
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

            extern crate proto as _proto;
            extern crate bytes as _bytes;

            #[automatically_derived]
            impl _proto::Message for #ident {
                #[inline]
                fn encode_raw<B>(&self, buf: &mut B) where B: _bytes::BufMut {
                    #(#encode)*
                }

                #[inline]
                fn merge<B>(&mut self, buf: &mut _bytes::Take<B>) -> ::std::io::Result<()> where B: _bytes::Buf {
                    fn map_err(field: &str, cause: ::std::io::Error) -> ::std::io::Error {
                        ::std::io::Error::new(cause.kind(),
                                              format!(concat!("failed to decode field ",
                                                              stringify!(#ident),
                                                              ".{}: {}"),
                                                      field, cause))
                    }

                    while _bytes::Buf::has_remaining(buf) {
                        let (tag, wire_type) = _proto::encoding::decode_key(buf)?;
                        match tag {
                            #(#merge)*
                            _ => _proto::encoding::skip_field(wire_type, buf)?,
                        }
                    }
                    Ok(())
                }

                #[inline]
                fn encoded_len(&self) -> usize {
                    0 #(+ #encoded_len)*
                }
            }

            #methods

            #[automatically_derived]
            impl Default for #ident {
                fn default() -> #ident {
                    #ident {
                        #(#default)*
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

#[proc_macro_derive(Enumeration, attributes(proto))]
pub fn enumeration(input: TokenStream) -> TokenStream {
    let syn::DeriveInput { ident, generics, body, .. } =
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

    let default = variants[0].0.clone();

    let dummy_const = Ident::new(format!("_IMPL_ENUMERATION_FOR_{}", ident));
    let is_valid = variants.iter().map(|&(_, ref value)| quote!(#value => true));
    let from = variants.iter().map(|&(ref variant, ref value)| quote!(#value => ::std::option::Option::Some(#ident::#variant)));

    let is_valid_doc = format!("Returns `true` if `value` is a variant of `{}`.", ident);
    let from_i32_doc = format!("Converts an `i32` to a `{}`, or `None` if `value` is not a valid variant.", ident);

    let expanded = quote! {
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const #dummy_const: () = {
            extern crate bytes as _bytes;
            extern crate proto as _proto;

            #[automatically_derived]
            impl #ident {

                #[doc=#is_valid_doc]
                pub fn is_valid(value: i32) -> bool {
                    match value {
                        #(#is_valid,)*
                        _ => false,
                    }
                }

                #[doc=#from_i32_doc]
                pub fn from_i32(value: i32) -> ::std::option::Option<#ident> {
                    match value {
                        #(#from,)*
                        _ => ::std::option::Option::None,
                    }
                }
            }

            #[automatically_derived]
            impl ::std::default::Default for #ident {
                fn default() -> #ident {
                    #ident::#default
                }
            }

            #[automatically_derived]
            impl ::std::convert::From<#ident> for i32 {
                fn from(value: #ident) -> i32 {
                    value as i32
                }
            }
        };
    };

    expanded.parse().unwrap()
}

fn try_oneof(input: TokenStream) -> Result<TokenStream> {
    let syn::DeriveInput { ident, generics, body, .. } = syn::parse_derive_input(&input.to_string())?;

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
        let variant_ident = variant.ident;
        let attrs = variant.attrs;
        if let syn::VariantData::Tuple(fields) = variant.data {
            if fields.len() != 1 {
                bail!("invalid oneof variant {}::{}: oneof variants must have a single field",
                      ident, variant_ident);
            }
            match Field::new(attrs) {
                Ok(Some(field)) => Ok((variant_ident, field)),
                Ok(None) => bail!("invalid oneof variant {}::{}: oneof variants may not be ignored",
                                  ident, variant_ident),
                Err(err) => bail!("invalid oneof variant {}::{}: {}", ident, variant_ident, err),
            }
        } else {
            bail!("invalid oneof variant {}::{}: oneof variants must have a single field",
                  ident, variant_ident);
        }
    }).collect::<Result<Vec<(Ident, Field)>>>()?;

    let mut tags = fields.iter().flat_map(|&(ref variant_ident, ref field)| -> Result<u32> {
        if field.tags().len() > 1 {
            bail!("invalid oneof variant {}::{}: oneof variants may only have a single tag",
                  ident, variant_ident);
        }
        Ok(field.tags()[0])
    }).collect::<Vec<_>>();
    tags.sort();
    tags.dedup();
    if tags.len() != fields.len() {
        panic!("invalid oneof {}: variants have duplicate tags", ident);
    }

    let dummy_const = Ident::new(format!("_IMPL_ONEOF_FOR_{}", ident));

    let encode = fields.iter().map(|&(ref variant_ident, ref field)| {
        let encode = field.encode(&Ident::new("*value"));
        quote!(#ident::#variant_ident(ref value) => #encode)
    });

    let merge = fields.iter().map(|&(ref variant_ident, ref field)| {
        let tag = field.tags()[0];
        let merge = field.merge(&Ident::new("value"));
        quote! {
            #tag => {
                let mut value = ::std::default::Default::default();
                #merge.map(|_| *field = ::std::option::Option::Some(#ident::#variant_ident(value)))
            }
        }
    });

    let encoded_len = fields.iter().map(|&(ref variant_ident, ref field)| {
        let encoded_len = field.encoded_len(&Ident::new("*value"));
        quote!(#ident::#variant_ident(ref value) => #encoded_len)
    });

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

            impl #ident {
                pub fn encode<B>(&self, buf: &mut B) where B: _bytes::BufMut {
                    match *self {
                        #(#encode,)*
                    }
                }

                pub fn merge<B>(field: &mut ::std::option::Option<#ident>,
                                tag: u32,
                                wire_type: _proto::encoding::WireType,
                                buf: &mut _bytes::Take<B>)
                                -> ::std::io::Result<()>
                where B: _bytes::Buf {
                    match tag {
                        #(#merge,)*
                        _ => unreachable!(concat!("invalid ", stringify!(#ident), " tag: {}"), tag),
                    }
                }

                pub fn encoded_len(&self) -> usize {
                    match *self {
                        #(#encoded_len,)*
                    }
                }
            }
        };
    };

    expanded.parse::<TokenStream>().map_err(|err| Error::from(format!("{:?}", err)))
}

#[proc_macro_derive(Oneof, attributes(proto))]
pub fn oneof(input: TokenStream) -> TokenStream {
    try_oneof(input).unwrap()
}
