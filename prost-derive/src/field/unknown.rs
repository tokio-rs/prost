use anyhow::{bail, Error};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Meta;

use crate::field::{set_bool, word_attr};

#[derive(Clone)]
pub struct Field {}

impl Field {
    pub fn new(attrs: &[Meta]) -> Result<Option<Field>, Error> {
        let mut unknown = false;
        let mut unknown_attrs = Vec::new();

        for attr in attrs {
            if word_attr("unknown_fields", attr) {
                set_bool(&mut unknown, "Multiple unknown_fields in one message")?;
            } else {
                unknown_attrs.push(attr);
            }
        }

        if !unknown {
            return Ok(None);
        }

        match unknown_attrs.len() {
            0 => (),
            1 => bail!(
                "unknown attribute for unknown field set: {:?}",
                unknown_attrs[0]
            ),
            _ => bail!(
                "unknown attributes for unknown field set: {:?}",
                unknown_attrs
            ),
        }

        Ok(Some(Field {}))
    }

    pub fn encode(&self, ident: TokenStream) -> TokenStream {
        quote! {
            #ident.encode_raw(buf)
        }
    }

    pub fn merge(&self, ident: TokenStream) -> TokenStream {
        quote! {
            #ident.merge_field(tag, wire_type, buf, ctx)
        }
    }

    pub fn encoded_len(&self, ident: TokenStream) -> TokenStream {
        quote! {
            #ident.encoded_len()
        }
    }

    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        quote! {
            #ident.clear()
        }
    }
}
