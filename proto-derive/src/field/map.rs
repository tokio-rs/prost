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
    pub key_ty: KeyTy,
    pub value_ty: ValueTy,
    pub tag: u32,
}

impl Field {

    pub fn new(ident: &Ident, attrs: &[MetaItem]) -> Result<Option<Field>> {
        let mut types = None;
        let mut tag = None;

        for attr in attrs {
            if let Some(t) = tag_attr(attr)? {
                set_option(&mut tag, t, "duplicate tag attributes")?;
            } else if attr.name() == "map" {
                let (k, v) = match *attr {
                    MetaItem::NameValue(_, Lit::Str(ref ident, _)) => {
                        let mut items = ident.split(',');
                        let k = items.next().unwrap();
                        let v = match items.next() {
                            Some(k) => k,
                            None => bail!("invalid map attribute: must have key and value types"),
                        };
                        if items.next().is_some() {
                            bail!("invalid map attribute: {:?}", attr);
                        }
                        (k, v)
                    },
                    MetaItem::List(_, ref items) => {
                        // TODO(rustlang/rust#23121): slice pattern matching would make this much nicer.
                        if items.len() != 2 {
                            bail!("invalid map attribute: must contain key and value types");
                        }
                        let k = match &items[0] {
                            &NestedMetaItem::MetaItem(MetaItem::Word(ref k)) => k.as_ref(),
                            _ => bail!("invalid map attribute: key must be an identifier"),
                        };
                        let v = match &items[1] {
                            &NestedMetaItem::MetaItem(MetaItem::Word(ref v)) => v.as_ref(),
                            _ => bail!("invalid map attribute: value must be an identifier"),
                        };
                        (k, v)
                    },
                    _ => return Ok(None),
                };
                set_option(&mut types, (KeyTy::from_str(k)?, ValueTy::from_str(v)?),
                           "duplicate map type attribute")?;
            } else {
                return Ok(None);
            }
        }

        Ok(match (types, tag) {
            (Some((key_ty, val_ty)), Some(tag)) => {
                Some(Field {
                    ident: ident.clone(),
                    key_ty: key_ty,
                    value_ty: val_ty,
                    tag: tag
                })
            },
            _ => None
        })
    }

    pub fn encode(&self) -> Tokens {
        unimplemented!()
    }

    pub fn merge(&self, tag: &Ident, wire_type: &Ident) -> Tokens {
        unimplemented!()
    }
}

/// A map field type.
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum KeyTy {
    Int32,
    Int64,
    Uint32,
    Uint64,
    Sint32,
    Sint64,
    Fixed32,
    Fixed64,
    Sfixed32,
    Sfixed64,
    Bool,
    String,
}

impl KeyTy {
    fn from_str(s: &str) -> Result<KeyTy> {
        Ok(match scalar::Ty::from_str(s)? {
            scalar::Ty::Int32 => KeyTy::Int32,
            scalar::Ty::Int64 => KeyTy::Int64,
            scalar::Ty::Uint32 => KeyTy::Uint32,
            scalar::Ty::Uint64 => KeyTy::Uint64,
            scalar::Ty::Sint32 => KeyTy::Sint32,
            scalar::Ty::Sint64 => KeyTy::Sint64,
            scalar::Ty::Fixed32 => KeyTy::Fixed32,
            scalar::Ty::Fixed64 => KeyTy::Fixed64,
            scalar::Ty::Sfixed32 => KeyTy::Sfixed32,
            scalar::Ty::Sfixed64 => KeyTy::Sfixed64,
            scalar::Ty::Bool => KeyTy::Bool,
            scalar::Ty::String => KeyTy::String,
            _ => bail!("invalid map key type: {}", s),
        })
    }
}

/// A map value type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueTy {
    Scalar(scalar::Ty),
    Message,
}

impl ValueTy {
    fn from_str(s: &str) -> Result<ValueTy> {
        if let Ok(ty) = scalar::Ty::from_str(s) {
            Ok(ValueTy::Scalar(ty))
        } else if s.trim() == "message" {
            Ok(ValueTy::Message)
        } else {
            bail!("invalid map value type: {}", s);
        }
    }
}
