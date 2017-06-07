use syn::{
    Ident,
    MetaItem,
};
use quote::Tokens;

use error::*;
use field::{
    word_attr,
    tag_attr,
    set_option,
    set_bool,
    Label,
};

pub struct Field {
    pub label: Label,
    pub tag: u32,
}

impl Field {
    pub fn new(attrs: &[MetaItem]) -> Result<Option<Field>> {
        let mut message = false;
        let mut label = None;
        let mut tag = None;

        let mut unknown_attrs = Vec::new();

        for attr in attrs {
            if word_attr("message", attr) {
                set_bool(&mut message, "duplicate message attribute")?;
            } else if let Some(t) = tag_attr(attr)? {
                set_option(&mut tag, t, "duplicate tag attributes")?;
            } else if let Some(l) = Label::from_attr(attr) {
                set_option(&mut label, l, "duplicate label attributes")?;
            } else {
                unknown_attrs.push(attr);
            }
        }

        if !message {
            return Ok(None);
        }

        match unknown_attrs.len() {
            0 => (),
            1 => bail!("unknown attribute for message field: {:?}", unknown_attrs[0]),
            _ => bail!("unknown attributes for message field: {:?}", unknown_attrs),
        }

        let tag = match tag {
            Some(tag) => tag,
            None => bail!("message field is missing a tag attribute"),
        };

        Ok(Some(Field {
            label: label.unwrap_or(Label::Optional),
            tag: tag,
        }))
    }

    pub fn encode(&self, ident: &Ident) -> Tokens {
        let tag = self.tag;
        match self.label {
            Label::Optional => quote! {
                if let Some(ref msg) = #ident {
                    if msg != &Default::default() {
                        _proto::encoding::encode_message(#tag, msg, buf);
                    }
                }
            },
            Label::Required => quote! {
                _proto::encoding::encode_message(#tag, &#ident, buf);
            },
            Label::Repeated => quote! {
                for msg in &#ident {
                    _proto::encoding::encode_message(#tag, msg, buf);
                }
            },
        }
    }

    pub fn merge(&self, ident: &Ident) -> Tokens {
        match self.label {
            // TODO(rustlang/rust#39288): Use Option::get_or_insert_with when available:
            // _proto::encoding::merge_message(#ident.get_or_insert_with(Default::default), buf)
            Label::Optional => quote! {
                {
                    if #ident.is_none() {
                        #ident = Some(Default::default());
                    }
                    match #ident {
                        Some(ref mut msg) => _proto::encoding::merge_message(msg, buf),
                        _ => unreachable!(),
                    }
                }
            },
            Label::Required => quote! {
                _proto::encoding::merge_message(&mut #ident, buf)
            },
            Label::Repeated => quote! {
                _proto::encoding::merge_repeated_message(&mut #ident, buf)
            },
        }
    }

    pub fn encoded_len(&self, ident: &Ident) -> Tokens {
        let tag = self.tag;
        match self.label {
            Label::Optional => quote! {
                #ident.as_ref().map_or(0, |msg| _proto::encoding::encoded_len_message(#tag, msg))
            },
            Label::Required => quote! {
                _proto::encoding::encoded_len_message(#tag, &#ident)
            },
            Label::Repeated => quote! {
                _proto::encoding::encoded_len_repeated_message(#tag, &#ident)
            },
        }
    }
}
