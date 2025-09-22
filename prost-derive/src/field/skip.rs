use anyhow::{bail, Error};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Meta};

use crate::field::{set_bool, word_attr};

#[derive(Clone)]
pub struct Field;

impl Field {
    pub fn new(attrs: &[Meta]) -> Result<Option<Field>, Error> {
        let mut skip = false;
        let mut unknown_attrs = Vec::new();

        for attr in attrs {
            if word_attr("skip", attr) {
                set_bool(&mut skip, "duplicate ignore attribute")?;
            } else {
                unknown_attrs.push(attr);
            }
        }

        if !skip {
            return Ok(None);
        }

        if !unknown_attrs.is_empty() {
            bail!(
                "unknown attribute(s) for ignored field: #[prost({})]",
                quote!(#(#unknown_attrs),*)
            );
        }

        Ok(Some(Field))
    }

    /// Returns a statement which non-ops, since the field is ignored.
    pub fn encode(&self, _: TokenStream) -> TokenStream {
        quote!()
    }

    /// Returns an expression which evaluates to the default value of the ignored field.
    pub fn merge(&self, ident: TokenStream) -> TokenStream {
        quote!(#ident.get_or_insert_with(::core::default::Default::default))
    }

    /// Returns an expression which evaluates to 0
    pub fn encoded_len(&self, _: TokenStream) -> TokenStream {
        quote!(0)
    }

    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        quote!(#ident = ::core::default::Default::default)
    }
}