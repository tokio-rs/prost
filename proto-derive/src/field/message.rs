use syn::{
    Attribute,
    Ident,
    Lit,
    MetaItem,
    NestedMetaItem,
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
    pub ident: Ident,
    pub label: Label,
    pub tag: u32,
}

impl Field {
    pub fn new(ident: &Ident, attrs: &[MetaItem]) -> Result<Option<Field>> {
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
            ident: ident.clone(),
            label: label.unwrap_or(Label::Optional),
            tag: tag,
        }))
    }

    pub fn encode(&self) -> Tokens {
        let ident = &self.ident;
        let tag = self.tag;
        match self.label {
            Label::Optional => quote! {
                if let Some(ref msg) = self.#ident {
                    _proto::encoding::encode_key(#tag, _proto::encoding::WireType::LengthDelimited, buf);
                    _proto::encoding::encode_varint(msg.encoded_len() as u64, buf);
                    msg.encode_raw(buf);
                }
            },
            Label::Required => quote! {
                _proto::encoding::encode_key(#tag, _proto::encoding::WireType::LengthDelimited, buf);
                _proto::encoding::encode_varint(self.#ident.encoded_len() as u64, buf);
                self.#ident.encode_raw(buf);
            },
            Label::Repeated => quote! {
                for msg in &self.#ident {
                    _proto::encoding::encode_key(#tag, _proto::encoding::WireType::LengthDelimited, buf);
                    _proto::encoding::encode_varint(msg.encoded_len() as u64, buf);
                    msg.encode_raw(buf);
                }
            },
        }
    }

    pub fn merge(&self) -> Tokens {
        let ident = &self.ident;
        let tag = self.tag;
        match self.label {
            // TODO(rustlang/rust#39288): Use Option::get_or_insert_with when available:
            // _proto::encoding::merge_message(self.#ident.get_or_insert_with(Default::default), buf)
            Label::Optional => quote! {
                {
                    if self.#ident.is_none() {
                        self.#ident = Some(Default::default());
                    }
                    match self.#ident {
                        Some(ref mut msg) => _proto::encoding::merge_message(msg, buf),
                        _ => unreachable!(),
                    }
                }
            },
            Label::Required => quote! {
                _proto::encoding::merge_message(&mut self.#ident, buf)
            },
            Label::Repeated => quote! {
                _proto::encoding::merge_repeated_message(&mut self.#ident, buf)
            },
        }
    }

    pub fn encoded_len(&self) -> Tokens {
        let ident = &self.ident;
        let tag = self.tag;
        match self.label {
            Label::Optional => quote! {
                self.#ident.as_ref().map_or(0, |msg| _proto::encoding::encoded_len_message(#tag, msg))
            },
            Label::Required => quote! {
                _proto::encoding::encoded_len_message(#tag, &self.#ident)
            },
            Label::Repeated => quote! {
                _proto::encoding::encoded_len_repeated_message(#tag, &self.#ident)
            },
        }
    }
}
