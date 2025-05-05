use anyhow::{bail, Error};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::Meta;

use crate::field::set_option;

use super::{set_bool, tag_attr, word_attr, Label};

#[derive(Clone)]
pub struct Field {
    pub label: Label,
    pub tag: u32,
}

impl Field {
    pub fn new(attrs: &[Meta], inferred_tag: Option<u32>) -> Result<Option<Field>, Error> {
        let mut enum_typed = false;
        let mut label = None;
        let mut tag = None;
        let mut unknown_attrs = Vec::new();

        for attr in attrs {
            if word_attr("enumeration_typed", attr) {
                set_bool(&mut enum_typed, "duplicate enumeration_typed attribute")?;
            } else if let Some(t) = tag_attr(attr)? {
                set_option(&mut tag, t, "duplicate tag attributes")?;
            } else if let Some(l) = Label::from_attr(attr) {
                set_option(&mut label, l, "duplicate label attributes")?;
            } else {
                unknown_attrs.push(attr);
            }
        }

        if !enum_typed {
            return Ok(None);
        }

        if !unknown_attrs.is_empty() {
            bail!(
                "unknown attribute(s) for enumeration_typed field: #[prost({})]",
                quote!(#(#unknown_attrs),*)
            );
        }

        let tag = match tag.or(inferred_tag) {
            Some(tag) => tag,
            None => bail!("enumeration_typed field is missing a tag attribute"),
        };

        Ok(Some(Field {
            label: label.unwrap_or(Label::Required),
            tag,
        }))
    }

    pub fn new_oneof(attrs: &[Meta]) -> Result<Option<Field>, Error> {
        if let Some(mut field) = Field::new(attrs, None)? {
            if let Some(attr) = attrs.iter().find(|attr| Label::from_attr(attr).is_some()) {
                bail!(
                    "invalid attribute for oneof field: {}",
                    attr.path().into_token_stream()
                );
            }
            field.label = Label::Required;
            Ok(Some(field))
        } else {
            Ok(None)
        }
    }

    pub fn encode(&self, ident: TokenStream) -> TokenStream {
        let tag = self.tag;
        match self.label {
            Label::Optional => quote! {
                if let Some(ref msg) = #ident {
                    ::prost::encoding::int32::encode(#tag, &(*msg as i32), buf);
                }
            },
            Label::Required => quote! {
                ::prost::encoding::int32::encode(#tag, &(#ident as i32), buf);
            },
            Label::Repeated => quote! {
                for msg in &#ident {
                    ::prost::encoding::int32::encode(#tag, &(*msg as i32), buf);
                }
            },
        }
    }

    pub fn merge(&self, ident: TokenStream) -> TokenStream {
        match self.label {
            Label::Optional => quote! {
                ::prost::encoding::int32::merge(wire_type,
                                              #ident.map(|msg| msg as i32).get_or_insert_with(::core::default::Default::default),
                                              buf,
                                              ctx)
            },
            Label::Required => quote! {
                ::prost::encoding::int32::merge(wire_type, &mut (*#ident as i32), buf, ctx)
            },
            Label::Repeated => quote! {
                ::prost::encoding::int32::merge_repeated(wire_type, &(*#ident as i32), buf, ctx)
            },
        }
    }

    pub fn encoded_len(&self, ident: TokenStream) -> TokenStream {
        let tag = self.tag;
        match self.label {
            Label::Optional => quote! {
                #ident.map_or(0, |msg| ::prost::encoding::int32::encoded_len(#tag, &(msg as i32)))
            },
            Label::Required => quote! {
                ::prost::encoding::int32::encoded_len(#tag, &(#ident as i32))
            },
            Label::Repeated => quote! {
                ::prost::encoding::int32::encoded_len_repeated(#tag, &(#ident as i32))
            },
        }
    }

    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        match self.label {
            Label::Optional => quote!(#ident = ::core::option::Option::None),
            Label::Required => quote!(#ident = ::core::default::Default::default()),
            Label::Repeated => quote!(#ident = ::core::default::Default::default()),
        }
    }
}
