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

#[derive(Debug)]
pub enum MapTy {
    HashMap,
    BTreeMap,
}

impl MapTy {
    fn from_str(s: &str) -> Option<MapTy> {
        match s {
            "map" | "hash_map" => Some(MapTy::HashMap),
            "btree_map" => Some(MapTy::BTreeMap),
            _ => None,
        }
    }

    fn module(&self) -> Ident {
        match *self {
            MapTy::HashMap => Ident::new("hash_map"),
            MapTy::BTreeMap => Ident::new("btree_map"),
        }
    }
}

pub struct Field {
    pub map_ty: MapTy,
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
            } else if let Some(map_ty) = MapTy::from_str(attr.name()) {
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
                set_option(&mut types, (map_ty, key_ty_from_str(k)?, ValueTy::from_str(v)?),
                           "duplicate map type attribute")?;
            } else {
                return Ok(None);
            }
        }

        Ok(match (types, tag) {
            (Some((map_ty, key_ty, val_ty)), Some(tag)) => {
                Some(Field {
                    map_ty: map_ty,
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
        let ke = Ident::new(format!("_prost::encoding::{}::encode", self.key_ty.encode_as()));
        let kl = Ident::new(format!("_prost::encoding::{}::encoded_len", self.key_ty.encode_as()));
        let module = self.map_ty.module();
        match self.value_ty {
            ValueTy::Scalar(scalar::Ty::Enumeration(ref ty)) => {
                let default = Ident::new(format!("{}::default() as i32", ty));
                quote! {
                    _prost::encoding::#module::encode_with_default(#ke, #kl,
                                                                   _prost::encoding::int32::encode,
                                                                   _prost::encoding::int32::encoded_len,
                                                                   &(#default),
                                                                   #tag, &#ident, buf);
                }
            },
            ValueTy::Scalar(ref value_ty) => {
                let ve = Ident::new(format!("_prost::encoding::{}::encode", value_ty.encode_as()));
                let vl = Ident::new(format!("_prost::encoding::{}::encoded_len", value_ty.encode_as()));
                quote! {
                    _prost::encoding::#module::encode(#ke, #kl, #ve, #vl,
                                                      #tag, &#ident, buf);
                }
            },
            ValueTy::Message => {
                quote! {
                    _prost::encoding::#module::encode(#ke, #kl,
                                                      _prost::encoding::message::encode,
                                                      _prost::encoding::message::encoded_len,
                                                      #tag, &#ident, buf);
                }
            },
        }
    }

    /// Returns an expression which evaluates to the result of merging a decoded key value pair
    /// into the map.
    pub fn merge(&self, ident: &Ident) -> Tokens {
        let km = Ident::new(format!("_prost::encoding::{}::merge", self.key_ty.encode_as()));
        let module = self.map_ty.module();
        match self.value_ty {
            ValueTy::Scalar(scalar::Ty::Enumeration(ref ty)) => {
                let default = Ident::new(format!("{}::default() as i32", ty));
                quote! {
                    _prost::encoding::#module::merge_with_default(#km, _prost::encoding::int32::merge,
                                                                  #default, &mut #ident, buf)
                }
            },
            ValueTy::Scalar(ref value_ty) => {
                let vm = Ident::new(format!("_prost::encoding::{}::merge", value_ty.encode_as()));
                quote!(_prost::encoding::#module::merge(#km, #vm, &mut #ident, buf))
            },
            ValueTy::Message => {
                quote!(_prost::encoding::#module::merge(#km, _prost::encoding::message::merge,
                                                        &mut #ident, buf))
            },
        }
    }

    /// Returns an expression which evaluates to the encoded length of the map.
    pub fn encoded_len(&self, ident: &Ident) -> Tokens {
        let tag = self.tag;
        let kl = Ident::new(format!("_prost::encoding::{}::encoded_len", self.key_ty.encode_as()));
        let module = self.map_ty.module();
        match self.value_ty {
            ValueTy::Scalar(scalar::Ty::Enumeration(ref ty)) => {
                let default = Ident::new(format!("{}::default() as i32", ty));
                quote! {
                    _prost::encoding::#module::encoded_len_with_default(
                        #kl, _prost::encoding::int32::encoded_len,
                        &(#default), #tag, &#ident)
                }
            },
            ValueTy::Scalar(ref value_ty) => {
                let vl = Ident::new(format!("_prost::encoding::{}::encoded_len", value_ty.encode_as()));
                quote!(_prost::encoding::#module::encoded_len(#kl, #vl, #tag, &#ident))
            },
            ValueTy::Message => {
                quote!(_prost::encoding::#module::encoded_len(#kl, _prost::encoding::message::encoded_len,
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
            let take_ref = Ident::new(if self.key_ty.is_numeric() { "&" } else { "" });

            Some(quote! {
                pub fn #get(&self, key: #key_ref_ty) -> ::std::option::Option<#ty> {
                    self.#ident.get(#take_ref key).cloned().and_then(#ty::from_i32)
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
