use anyhow::{bail, ensure, Error};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Expr, Meta};

use crate::field::{
    as_msg_attr,
    from_msg_attr,
    merge_msg_attr,
    set_bool,
    set_option,
    tag_attr,
    to_msg_attr,
    word_attr,
    Label,
};

#[derive(Clone)]
pub struct Field {
    pub label: Label,
    pub tag: u32,
    pub as_msg: Option<Expr>,
    pub to_msg: Option<Expr>,
    pub from_msg: Option<Expr>,
    pub merge_msg: Option<Expr>,
}

impl Field {
    pub fn new(attrs: &[Meta], inferred_tag: Option<u32>) -> Result<Option<Field>, Error> {
        let mut message = false;
        let mut label = None;
        let mut tag = None;
        let mut as_msg = None;
        let mut to_msg = None;
        let mut from_msg = None;
        let mut merge_msg = None;
        let mut boxed = false;

        let mut unknown_attrs = Vec::new();

        for attr in attrs {
            if word_attr("message", attr) {
                set_bool(&mut message, "duplicate message attribute")?;
            } else if word_attr("boxed", attr) {
                set_bool(&mut boxed, "duplicate boxed attribute")?;
            } else if let Some(t) = tag_attr(attr)? {
                set_option(&mut tag, t, "duplicate tag attributes")?;
            } else if let Some(l) = Label::from_attr(attr) {
                set_option(&mut label, l, "duplicate label attributes")?;
            } else if let Some(a) = as_msg_attr(attr)? {
                set_option(&mut as_msg, a, "duplicate as_msg attributes")?;
            } else if let Some(t) = to_msg_attr(attr)? {
                set_option(&mut to_msg, t, "duplicate to_msg attributes")?;
            } else if let Some(f) = from_msg_attr(attr)? {
                set_option(&mut from_msg, f, "duplicate from_msg attributes")?;
            } else if let Some(m) = merge_msg_attr(attr)? {
                set_option(&mut merge_msg, m, "duplicate merge_msg attributes")?;
            } else {
                unknown_attrs.push(attr);
            }
        }

        if !message {
            return Ok(None);
        }

        match unknown_attrs.len() {
            0 => (),
            1 => bail!(
                "unknown attribute for message field: {:?}",
                unknown_attrs[0]
            ),
            _ => bail!("unknown attributes for message field: {:?}", unknown_attrs),
        }

        let tag = match tag.or(inferred_tag) {
            Some(tag) => tag,
            None => bail!("message field is missing a tag attribute"),
        };
        
        ensure!(
            (as_msg.is_none() && to_msg.is_none()) || (from_msg.is_some() || merge_msg.is_some()),
            "missing from_msg or merge_msg attribute",
        );

        ensure!(
            (from_msg.is_none() && merge_msg.is_none()) || (as_msg.is_some() || to_msg.is_some()),
            "missing as_msg or to_msg attribute",
        );

        Ok(Some(Field {
            label: label.unwrap_or(Label::Optional),
            tag,
            as_msg,
            to_msg,
            from_msg,
            merge_msg,
        }))
    }

    pub fn new_oneof(attrs: &[Meta]) -> Result<Option<Field>, Error> {
        if let Some(mut field) = Field::new(attrs, None)? {
            ensure!(
                field.as_msg.is_none() && field.to_msg.is_none()
                    && field.from_msg.is_none() && field.merge_msg.is_none(),
                "oneof messages cannot have as_msg, to_msg, from_msg, or merge_msg attributes",
            );

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
            Label::Optional => {
                let msg = match (&self.as_msg, &self.to_msg) {
                    (Some(as_msg), _) => quote!(#as_msg(&#ident)),
                    (None, Some(to_msg)) => quote!(#to_msg(&#ident).as_ref()),
                    (None, None) => quote!(#ident.as_ref()),
                };

                quote! {
                    if let ::core::option::Option::Some(value) = #msg {
                        ::prost::encoding::message::encode(#tag, value, buf);
                    }
                }
            }
            Label::Required => {
                let msg = match (&self.as_msg, &self.to_msg) {
                    (Some(as_msg), _) => quote!(#as_msg(&#ident)),
                    (None, Some(to_msg)) => quote!(&#to_msg(&#ident)),
                    (None, None) => quote!(&#ident),
                };

                quote! {
                    ::prost::encoding::message::encode(#tag, #msg, buf);
                }
            }
            Label::Repeated => {
                let msg = match (&self.as_msg, &self.to_msg) {
                    (Some(as_msg), _) => quote!(#as_msg(value)),
                    (None, Some(to_msg)) => quote!(&#to_msg(value)),
                    (None, None) => quote!(value),
                };

                quote! {
                    #ident.iter().for_each(|value| {
                        ::prost::encoding::message::encode(#tag, #msg, buf);
                    });
                }
            }
        }
    }

    pub fn merge(&self, ident: TokenStream) -> TokenStream {
        match self.label {
            Label::Optional => match (&self.from_msg, &self.merge_msg) {
                (_, Some(merge_msg)) => quote! {{
                    let mut msg = Default::default();
                    ::prost::encoding::message::merge(wire_type, &mut msg, buf, ctx)
                        .map(|_| #merge_msg(#ident, Some(msg)))
                }},
                (Some(from_msg), None) => quote! {{
                    let mut msg = Default::default();
                    ::prost::encoding::message::merge(wire_type, &mut msg, buf, ctx)
                        .map(|_| *#ident = #from_msg(Some(msg)))
                }},
                (None, None) => quote! {
                    ::prost::encoding::message::merge(
                        wire_type,
                        #ident.get_or_insert_with(Default::default),
                        buf,
                        ctx,
                    )
                },
            },
            Label::Required => match (&self.from_msg, &self.merge_msg) {
                (_, Some(merge_msg)) => quote! {{
                    let mut msg = Default::default();
                    ::prost::encoding::message::merge(wire_type, &mut msg, buf, ctx)
                        .map(|_| #merge_msg(#ident, msg))
                }},
                (Some(from_msg), None) => quote! {{
                    let mut msg = Default::default();
                    ::prost::encoding::message::merge(wire_type, &mut msg, buf, ctx)
                        .map(|_| *#ident = #from_msg(msg))
                }},
                (None, None) => quote! {
                    ::prost::encoding::message::merge(wire_type, #ident, buf, ctx)
                }
            },
            Label::Repeated => match (&self.from_msg, &self.merge_msg) {
                (Some(from_msg), _) => quote! {{
                    let mut msg = Default::default();
                    ::prost::encoding::message::merge(wire_type, &mut msg, buf, ctx)
                        .map(|_| #ident.push(#from_msg(msg)))
                }},
                (None, Some(merge_msg)) => quote! {{
                    let mut msg = Default::default();
                    ::prost::encoding::message::merge(wire_type, &mut msg, buf, ctx).map(|_| {
                        let mut val = Default::default();
                        #merge_msg(&mut val, msg);
                        #ident.push(val);
                    })
                }},
                (None, None) => quote! {{
                    ::prost::encoding::message::merge_repeated(wire_type, #ident, buf, ctx)
                }}
            }
        }
    }

    pub fn encoded_len(&self, ident: TokenStream) -> TokenStream {
        let tag = self.tag;

        match self.label {
            Label::Optional => {
                let msg = match (&self.as_msg, &self.to_msg) {
                    (Some(as_msg), _) => quote!(#as_msg(&#ident)),
                    (None, Some(to_msg)) => quote!(#to_msg(&#ident).as_ref()),
                    (None, None) => quote!(#ident.as_ref()),
                };

                quote! {
                    #msg.map_or(0, |value| ::prost::encoding::message::encoded_len(#tag, value))
                }
            }
            Label::Required => {
                let msg = match (&self.as_msg, &self.to_msg) {
                    (Some(as_msg), _) => quote!(#as_msg(&#ident)),
                    (None, Some(to_msg)) => quote!(&#to_msg(&#ident)),
                    (None, None) => quote!(&#ident),
                };

                quote! {
                    ::prost::encoding::message::encoded_len(#tag, #msg)
                }
            }
            Label::Repeated => {
                let msg = match (&self.as_msg, &self.to_msg) {
                    (Some(as_msg), _) => quote!(#as_msg(value)),
                    (None, Some(to_msg)) => quote!(&#to_msg(value)),
                    (None, None) => quote!(value),
                };

                quote! {
                    #ident.iter().map(|value| {
                        ::prost::encoding::message::encoded_len(#tag, #msg)
                    }).sum::<usize>()
                }
            }
        }
    }

    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        match self.label {
            Label::Optional => match (&self.from_msg, &self.merge_msg) {
                (_, Some(merge_msg)) => quote! {
                    #merge_msg(&mut #ident, ::core::option::Option::None)
                },
                (Some(from_msg), None) => quote! {
                    #ident = #from_msg(::core::option::Option::None)
                },
                (None, None) => quote! {
                    #ident = ::core::option::Option::None
                }
            },
            Label::Required => match (&self.from_msg, &self.merge_msg) {
                (_, Some(merge_msg)) => quote!(#merge_msg(&mut #ident, Default::default())),
                (Some(from_msg), None) => quote!(#ident = #from_msg(Default::default())),
                (None, None) => quote!(#ident.clear()),
            }
            Label::Repeated => quote!(#ident.clear()),
        }
    }

    pub fn debug(&self, ident: TokenStream) -> TokenStream {
        match (&self.as_msg, &self.to_msg) {
            (Some(as_msg), _) => quote!(#as_msg(&#ident)),
            (None, Some(to_msg)) => quote!(&#to_msg(&#ident)),
            (None, None) => quote!(&#ident),
        }
    }
}
