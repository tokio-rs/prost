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
    scalar,
    tag_attr,
    set_option,
};

pub struct Field {
    pub ident: Ident,
    pub tags: Vec<u32>,
}

impl Field {
    pub fn new(ident: &Ident, attrs: &[MetaItem]) -> Result<Option<Field>> {
        unimplemented!()
    }
}
