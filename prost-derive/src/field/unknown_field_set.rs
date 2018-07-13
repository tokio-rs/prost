use failure::Error;
use proc_macro2::TokenStream;
use syn::{
    Meta,
};

#[derive(Clone)]
pub struct Field {
}

impl Field {
    pub fn new(attrs: &[Meta]) -> Result<Option<Field>, Error> {
        if attrs.len() != 1 || attrs[0].name() != "unknown_field_set" {
        	bail!("invalid format for unknown_field_set annotation");
        }

        Ok(Some(Field {}))
    }

    /// Returns a statement which encodes the field.
    pub fn encode(&self, _ident: TokenStream) -> TokenStream {
        quote! {}
    }

    /// Returns an expression which evaluates to the result of decoding the field.
    pub fn merge(&self, _ident: TokenStream) -> TokenStream {
        quote! {}
    }

    /// Returns an expression which evaluates to the encoded length of the field.
    pub fn encoded_len(&self, _ident: TokenStream) -> TokenStream {
        quote! { 0 }
    }

    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        quote!(#ident = ::std::default::Default::default())
    }
}
