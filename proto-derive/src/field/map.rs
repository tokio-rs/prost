use syn::{
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
    pub key_ty: scalar::Ty,
    pub value_ty: ValueTy,
    pub tag: u32,
}

impl Field {

    pub fn new(attrs: &[MetaItem]) -> Result<Option<Field>> {
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
                    key_ty: key_ty,
                    value_ty: val_ty,
                    tag: tag
                })
            },
            _ => None
        })
    }

    pub fn new_oneof(attrs: &[MetaItem]) -> Result<Option<Field>> {
        Field::new(attrs)
    }

    /// Returns a statement which encodes the map field.
    pub fn encode(&self, ident: &Ident) -> Tokens {
        let tag = self.tag;
        let ke = Ident::new(format!("_proto::encoding::{}::encode", self.key_ty.encode_as()));
        let kl = Ident::new(format!("_proto::encoding::{}::encoded_len", self.key_ty.encode_as()));
        match self.value_ty {
            ValueTy::Scalar(scalar::Ty::Enumeration(ref ty)) => {
                let default = Ident::new(format!("{}::default() as i32", ty));
                quote! {
                    _proto::encoding::map::encode_with_default(#ke, #kl,
                                                               _proto::encoding::int32::encode,
                                                               _proto::encoding::int32::encoded_len,
                                                               &(#default),
                                                               #tag, &#ident, buf);
                }
            },
            ValueTy::Scalar(ref value_ty) => {
                let ve = Ident::new(format!("_proto::encoding::{}::encode", value_ty.encode_as()));
                let vl = Ident::new(format!("_proto::encoding::{}::encoded_len", value_ty.encode_as()));
                quote! {
                    _proto::encoding::map::encode(#ke, #kl, #ve, #vl,
                                                  #tag, &#ident, buf);
                }
            },
            ValueTy::Message => {
                quote! {
                    _proto::encoding::map::encode(#ke, #kl,
                                                  _proto::encoding::message::encode,
                                                  _proto::encoding::message::encoded_len,
                                                  #tag, &#ident, buf);
                }
            },
        }
    }

    /// Returns an expression which evaluates to the result of merging a decoded key value pair
    /// into the map.
    pub fn merge(&self, ident: &Ident) -> Tokens {
        let km = Ident::new(format!("_proto::encoding::{}::merge", self.key_ty.encode_as()));
        match self.value_ty {
            ValueTy::Scalar(scalar::Ty::Enumeration(ref ty)) => {
                let default = Ident::new(format!("{}::default() as i32", ty));
                quote! {
                    _proto::encoding::map::merge_with_default(#km, _proto::encoding::int32::merge,
                                                              #default, &mut #ident, buf)
                }
            },
            ValueTy::Scalar(ref value_ty) => {
                let vm = Ident::new(format!("_proto::encoding::{}::merge", value_ty.encode_as()));
                quote!(_proto::encoding::map::merge(#km, #vm, &mut #ident, buf))
            },
            ValueTy::Message => {
                quote!(_proto::encoding::map::merge(#km, _proto::encoding::message::merge,
                                                    &mut #ident, buf))
            },
        }
    }

    /// Returns an expression which evaluates to the encoded length of the map.
    pub fn encoded_len(&self, ident: &Ident) -> Tokens {
        let tag = self.tag;
        let kl = Ident::new(format!("_proto::encoding::{}::encoded_len", self.key_ty.encode_as()));
        match self.value_ty {
            ValueTy::Scalar(scalar::Ty::Enumeration(ref ty)) => {
                let default = Ident::new(format!("{}::default() as i32", ty));
                quote! {
                    _proto::encoding::map::encoded_len_with_default(
                        #kl, _proto::encoding::int32::encoded_len,
                        &(#default), #tag, &#ident)
                }
            },
            ValueTy::Scalar(ref value_ty) => {
                let vl = Ident::new(format!("_proto::encoding::{}::encoded_len", value_ty.encode_as()));
                quote!(_proto::encoding::map::encoded_len(#kl, #vl, #tag, &#ident))
            },
            ValueTy::Message => {
                quote!(_proto::encoding::map::encoded_len(#kl, _proto::encoding::message::encoded_len,
                                                          #tag, &#ident))
            },
        }
    }

    /// Returns methods to embed in the message.
    pub fn methods(&self, ident: &Ident) -> Option<Tokens> {
        if let ValueTy::Scalar(scalar::Ty::Enumeration(ref ty)) = self.value_ty {
            let key_ty = Ident::new(self.key_ty.rust_type());
            let key_ref_ty = Ident::new(self.key_ty.rust_ref_type());

            let get = Ident::new(format!("get_{}", ident));
            let insert = Ident::new(format!("insert_{}", ident));

            Some(quote! {
                pub fn #get(&self, key: &#key_ref_ty) -> ::std::option::Option<#ty> {
                    self.#ident.get(key).cloned().and_then(#ty::from_i32)
                }

                pub fn #insert(&mut self, key: #key_ty, value: #ty) -> ::std::option::Option<#ty> {
                    self.#ident.insert(key, value as i32).and_then(#ty::from_i32)
                }
            })
        } else {
            None
        }
    }
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
