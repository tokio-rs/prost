use crate::field::word_attr;
use anyhow::Error;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Meta;

/// A field representing the ExtensionSet type.
/// This field is use in Extendable messages to provide a seam for attempting to merge unknowns as
/// extensions before skipping them.
#[derive(Clone)]
pub struct Field {}

impl Field {
    pub fn new(attrs: &[Meta]) -> Result<Option<Field>, Error> {
        for attr in attrs {
            if word_attr("extension_set", attr) {
                return Ok(Some(Self {}));
            }
        }
        Ok(None)
    }

    pub fn encode(&self, ident: TokenStream) -> TokenStream {
        quote!(#ident.encode(buf))
    }

    pub fn encoded_len(&self, ident: TokenStream) -> TokenStream {
        quote!(#ident.encoded_len())
    }

    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        quote!(#ident.clear())
    }
}
