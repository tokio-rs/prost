use std::convert::TryFrom;
use std::fmt;

use anyhow::{anyhow, bail, Error};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{parse_str, Ident, Lit, LitByteStr, Meta, MetaList, MetaNameValue, NestedMeta, Path};

use crate::field::{set_bool, tag_attr, word_attr, Label};

pub fn encode_unknown(_ident: TokenStream) -> TokenStream {
    // quote! {
    //     if let Some(ref msg) = #ident {
    //         ::prost::encoding::message::encode(1000, &#ident, buf);
    //     }
    // }
    quote! {
        let y = 0;
    }
}

pub fn merge_unknown(_ident: TokenStream) -> TokenStream {
    // quote! {
    //     ::prost::encoding::message::merge(wire_type, #ident, buf, ctx)
    // }
    quote! {
        let rs: Result<(), ::prost::DecodeError> = Ok(());
        rs
    }
}

pub fn encoding_len_unknown(_ident: TokenStream) -> TokenStream {
    // #ident.as_ref().map_or(0, |msg| ::prost::encoding::message::encoded_len(1000, msg))
    // TODO(jason)
    quote! {
        1000
    }
}

pub fn clear_unknown(ident: TokenStream) -> TokenStream {
    quote! {
        #ident = ::prost::unknown::UnknownFields::default();
    }
}

pub fn matches_attrs(attrs: &[Meta]) -> Result<Option<()>, Error> {
    let mut is_unknown_fields = false;

    let mut unknown_attrs = Vec::new();

    for attr in attrs {
        if word_attr("unknown", attr) {
            set_bool(&mut is_unknown_fields, "duplicate group attributes")?;
        } else {
            unknown_attrs.push(attr);
        }
    }

    match unknown_attrs.len() {
        0 => (),
        1 => bail!("unknown attribute for group field: {:?}", unknown_attrs[0]),
        _ => bail!("unknown attributes for group field: {:?}", unknown_attrs),
    }

    if is_unknown_fields {
        Ok(Some(()))
    } else {
        Ok(None)
    }
}
