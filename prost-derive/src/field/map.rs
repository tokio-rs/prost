use failure::Error;
use syn::{
    Ident,
    Lit,
    MetaItem,
    NestedMetaItem,
};
use quote::Tokens;

use field::{
    scalar,
    tag_attr,
    set_option,
};

#[derive(Clone, Debug)]
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

fn fake_scalar(ty: scalar::Ty) -> scalar::Field {
    let kind = scalar::Kind::Plain(scalar::DefaultValue::new(&ty));
    scalar::Field {
        ty,
        kind,
        tag: 0, // Not used here
    }
}

#[derive(Clone)]
pub struct Field {
    pub map_ty: MapTy,
    pub key_ty: scalar::Ty,
    pub value_ty: ValueTy,
    pub tag: u32,
}

impl Field {

    pub fn new(attrs: &[MetaItem]) -> Result<Option<Field>, Error> {
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

    pub fn new_oneof(attrs: &[MetaItem]) -> Result<Option<Field>, Error> {
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

    pub fn clear(&self, ident: &Ident) -> Tokens {
        quote!(#ident.clear())
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

    /// Returns a newtype wrapper around the map, implementing nicer Debug
    ///
    /// The Debug tries to convert any enumerations met into the variants if possible, instead of
    /// outputting the raw numbers.
    pub fn debug(&self, wrapper_name: &Ident) -> Tokens {
        let type_name = match self.map_ty {
            MapTy::HashMap => Ident::new("HashMap"),
            MapTy::BTreeMap => Ident::new("BTreeMap"),
        };
        // A fake field for generating the debug wrapper
        let key_wrapper = fake_scalar(self.key_ty.clone()).debug(&Ident::new("KeyWrapper"));
        let key = Ident::new(self.key_ty.rust_type());
        let value_wrapper = self.value_ty.debug();
        let fmt = quote! {
            fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                #key_wrapper
                #value_wrapper
                let mut builder = f.debug_map();
                for (k, v) in self.0 {
                    builder.entry(&KeyWrapper(k), &ValueWrapper(v));
                }
                builder.finish()
            }
        };
        match self.value_ty {
            ValueTy::Scalar(ref ty) => {
                let value = Ident::new(ty.rust_type());
                quote! {
                    struct #wrapper_name<'a>(&'a ::std::collections::#type_name<#key, #value>);
                    impl<'a> ::std::fmt::Debug for #wrapper_name<'a> {
                        #fmt
                    }
                }
            },
            ValueTy::Message => quote! {
                struct #wrapper_name<'a, V: 'a>(&'a ::std::collections::#type_name<#key, V>);
                impl<'a, V> ::std::fmt::Debug for #wrapper_name<'a, V>
                where
                    V: ::std::fmt::Debug + 'a,
                {
                    #fmt
                }
            }
        }
    }
}

fn key_ty_from_str(s: &str) -> Result<scalar::Ty, Error> {
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
    fn from_str(s: &str) -> Result<ValueTy, Error> {
        if let Ok(ty) = scalar::Ty::from_str(s) {
            Ok(ValueTy::Scalar(ty))
        } else if s.trim() == "message" {
            Ok(ValueTy::Message)
        } else {
            bail!("invalid map value type: {}", s);
        }
    }

    /// Returns a newtype wrapper around the ValueTy for nicer debug.
    ///
    /// If the contained value is enumeration, it tries to convert it to the variant. If not, it
    /// just forwards the implementation.
    fn debug(&self) -> Tokens {
        match *self {
            ValueTy::Scalar(ref ty) => fake_scalar(ty.clone()).debug(&Ident::new("ValueWrapper")),
            ValueTy::Message => quote!(fn ValueWrapper<T>(v: T) -> T { v }),
        }
    }
}
