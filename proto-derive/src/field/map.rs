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
    pub key_ty: scalar::Ty,
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
                set_option(&mut types, (key_ty_from_str(k)?, ValueTy::from_str(v)?),
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

    /// Returns a statement which encodes the map field.
    pub fn encode(&self) -> Tokens {
        let tag = self.tag;
        let field = Ident::new(format!("self.{}", self.ident));
        match self.value_ty {
            ValueTy::Scalar(ref value_ty) => {
                let encode_fn = Ident::new(format!("_proto::encoding::encode_map_{}_{}",
                                                   self.key_ty.encode_as(),
                                                   value_ty.encode_as()));
                quote!(#encode_fn(#tag, &#field, buf);)
            },
            ValueTy::Message => {
                panic!("unimplemented: map field message values");
            },
        }
    }

    /// Returns an expression which evaluates to the result of merging a decoded key value pair
    /// into the map.
    pub fn merge(&self) -> Tokens {
        let field = Ident::new(format!("self.{}", self.ident));
        match self.value_ty {
            ValueTy::Scalar(ref value_ty) => {
                let merge_fn = Ident::new(format!("_proto::encoding::merge_map_{}_{}",
                                                   self.key_ty.encode_as(),
                                                   value_ty.encode_as()));
                quote!(#merge_fn(&mut #field, buf))
            },
            ValueTy::Message => {
                panic!("unimplemented: map field message values");
            },
        }
    }

    /// Returns an expression which evaluates to the encoded length of the map.
    pub fn encoded_len(&self) -> Tokens {
        let tag = self.tag;
        let field = Ident::new(format!("self.{}", self.ident));
        match self.value_ty {
            ValueTy::Scalar(ref value_ty) => {
                let encoded_len_fn = Ident::new(format!("_proto::encoding::encoded_len_map_{}_{}",
                                                        self.key_ty.encode_as(),
                                                        value_ty.encode_as()));
                quote!(#encoded_len_fn(#tag, &#field))
            },
            ValueTy::Message => {
                panic!("unimplemented: map field message values");
            },
        }
    }

    /*
impl <K, V, EK, EV> Field<(EK, EV)> for HashMap<K, V>
where K: default::Default + Eq + Hash + Key + Field<EK>,
      V: default::Default + Field<EV> {

    fn merge_from(&mut self, wire_type: WireType, r: &mut Read, limit: &mut usize) -> Result<()> {
        check_wire_type(WireType::LengthDelimited, wire_type)?;
        let len = <u64 as ScalarField>::read_from(r, limit)?;
        if len > usize::MAX as u64 {
            return Err(Error::new(ErrorKind::InvalidData,
                                  "map length overflows usize"));
        }
        check_limit(len as usize, limit)?;

        let mut key = None;
        let mut value = None;

        let mut limit = len as usize;
        while limit > 0 {
            let (wire_type, tag) = read_key_from(r, &mut limit)?;
            match tag {
                1 => {
                    let mut k = K::default();
                    <K as Field<EK>>::merge_from(&mut k, wire_type, r, &mut limit)?;
                    key = Some(k);
                },
                2 => {
                    let mut v = V::default();
                    <V as Field<EV>>::merge_from(&mut v, wire_type, r, &mut limit)?;
                    value = Some(v);
                },
                _ => return Err(Error::new(ErrorKind::InvalidData,
                                           format!("map entry contains unexpected field; tag: {:?}, wire type: {:?}",
                                                   tag, wire_type))),
            }
        }

        match (key, value) {
            (Some(key), Some(value)) => {
                self.insert(key, value);
            },
            (Some(_), None) => return Err(Error::new(ErrorKind::InvalidData,
                                                     "map entry is missing a key")),
            (None, Some(_)) => return Err(Error::new(ErrorKind::InvalidData,
                                                     "map entry is missing a value")),
            (None, None) => return Err(Error::new(ErrorKind::InvalidData,
                                                  "map entry is missing a key and a value")),
        }

        Ok(())
    }

    fn wire_len(&self, tag: u32) -> usize {
        self.iter().fold(key_len(tag), |acc, (key, value)| {
            acc + Field::<EK>::wire_len(key, 1) + Field::<EV>::wire_len(value, 2)
        })
    }
}
*/

}

fn key_ty_from_str(s: &str) -> Result<scalar::Ty> {
    let ty = scalar::Ty::from_str(s)?;
    match ty {
        scalar::Ty::Int32 | scalar::Ty::Int64 | scalar::Ty::Uint32 |
            scalar::Ty::Uint64 | scalar::Ty::Sint32 | scalar::Ty::Sint64 |
            scalar::Ty::Fixed32 | scalar::Ty::Fixed64 | scalar::Ty::Sfixed32 |
            scalar::Ty::Sfixed64 | scalar::Ty::Bool | scalar::Ty::String  => Ok(ty),
        _ => bail!("invalid map key type: {}", s),
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
