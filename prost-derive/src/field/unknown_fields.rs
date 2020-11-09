use anyhow::{bail, Error};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Meta;

#[derive(Clone)]
pub struct Field;

impl Field {
    pub fn new(attrs: &[Meta]) -> Result<Option<Field>, Error> {
        if let Some(attr) = attrs.iter().next() {
            if attr.path().is_ident("unknown_fields") {
                if attrs.len() > 1 {
                    bail!("invalid unknown_fields attribute(s): {:?}", &attrs[1..0]);
                }
                return Ok(Some(Field));
            }
        }
        Ok(None)
    }

    pub fn encode(&self, ident: TokenStream) -> TokenStream {
        quote! {
            for field in #ident.iter() {
                ::prost::encoding::bytes::encode(field.tag, &field.value, buf);
            }
        }
    }

    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        quote!(#ident.clear())
    }

    pub fn encoded_len(&self, ident: TokenStream) -> TokenStream {
        quote! {
            #ident
                .iter()
                .map(|field| ::prost::encoding::bytes::encoded_len(field.tag, &field.value))
                .sum::<usize>()
        }
    }

    pub fn merge(&self, _ident: TokenStream) -> TokenStream {
        // Adding unknown fields is handled separately by the decoding logic
        quote!(Result::<(), ::prost::DecodeError>::Ok(()))
    }
}
