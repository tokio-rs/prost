use syn::{
    Attribute,
    Ident,
    Lit,
    MetaItem,
    NestedMetaItem,
};
use quote::Tokens;

use error::*;
use field::Label;

pub struct Field {
    pub ident: Ident,
    pub ty: Ident,
    pub kind: Kind,
    pub tag: u32,
}

impl Field {
    pub fn new(ident: &Ident, attrs: &[MetaItem]) -> Result<Option<Field>> {
        Ok(None)
    }

    pub fn encode(&self) -> Tokens {
        let ident = &self.ident;
        let tag = &self.ident;
        quote! {
            let len = self.#ident.encoded_len();
            _proto::encoding::encode_key(#tag, _proto::encoding::WireType::Varint, buf);
            _proto::encoding::encode_varint(len as u64, buf);
            self.#ident.encode(buf)
        }
    }

    pub fn merge(&self, tag: &Ident, wire_type: &Ident) -> Tokens {
        unimplemented!()
    }
}

/// Message field types.
pub enum Kind {
    /// An optional message field.
    Optional,
    /// A required message field.
    Required,
    /// A repeated message field.
    Repeated,
}

