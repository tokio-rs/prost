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
    tags_attr,
    set_option,
    set_bool,
};

pub struct Field {
    pub ident: Ident,
    pub ty: Ident,
    pub tags: Vec<u32>,
}

impl Field {
    pub fn new(ident: &Ident, attrs: &[MetaItem]) -> Result<Option<Field>> {
        let mut ty = None;
        let mut tags = None;
        let mut unknown_attrs = Vec::new();

        for attr in attrs {
            if attr.name() == "oneof" {
                let t = match *attr {
                    MetaItem::NameValue(ref name, Lit::Str(ref ident, _)) => {
                        Ident::new(ident.as_ref())
                    },
                    MetaItem::List(ref name, ref items) if items.len() == 1 => {
                        // TODO(rustlang/rust#23121): slice pattern matching would make this much nicer.
                        if let NestedMetaItem::MetaItem(MetaItem::Word(ref ident)) = items[0] {
                            ident.clone()
                        } else {
                            bail!("invalid oneof attribute: item must be an identifier");
                        }
                    },
                    _ => bail!("invalid oneof attribute: {:?}", attr),
                };
                set_option(&mut ty, t, "duplicate oneof attribute")?;
            } else if let Some(t) = tags_attr(attr)? {
                set_option(&mut tags, t, "duplicate tags attributes")?;
            } else {
                unknown_attrs.push(attr);
            }
        }

        let ty = match ty {
            Some(ty) => ty,
            None => return Ok(None),
        };

        match unknown_attrs.len() {
            0 => (),
            1 => bail!("unknown attribute for message field: {:?}", unknown_attrs[0]),
            _ => bail!("unknown attributes for message field: {:?}", unknown_attrs),
        }

        let tags = match tags {
            Some(tags) => tags,
            None => bail!("oneof field is missing a tags attribute"),
        };

        Ok(Some(Field {
            ident: ident.clone(),
            ty: ty,
            tags: tags,
        }))
    }

    /// Returns a statement which encodes the oneof field.
    pub fn encode(&self) -> Tokens {
        let ident = &self.ident;
        quote! {
            if let Some(ref oneof) = self.#ident {
                oneof.encode(buf)
            }
        }
    }

    /// Returns an expression which evaluates to the result of decoding the oneof field.
    pub fn merge(&self) -> Tokens {
        let ty = &self.ty;
        let ident = &self.ident;
        quote! {
            match #ty::decode(tag, wire_type) {
                Ok(Some(value)) => {
                    self.#ident = Some(value);
                    Ok(())
                }
                Ok(None) => {
                    self.#ident = None;
                    _proto::encoding::skip_field(wire_type, buf)
                },
                Err(error) => Err(error),
            }
        }
    }

    /// Returns an expression which evaluates to the encoded length of the oneof field.
    pub fn encoded_len(&self) -> Tokens {
        let ty = &self.ty;
        let ident = &self.ident;
        quote! {
            self.#ident.as_ref().map_or(0, #ty::encoded_len)
        }
    }
}
