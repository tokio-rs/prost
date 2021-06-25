use anyhow::{bail, ensure, Error};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    AngleBracketedGenericArguments, GenericArgument, Ident, Lit, Meta, MetaNameValue, NestedMeta,
    Path, PathArguments, PathSegment, Type, TypePath,
};

use crate::field::{
    as_msg_attr, from_msg_attr, merge_msg_attr, scalar, set_option, tag_attr, to_msg_attr,
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
            MapTy::HashMap => Ident::new("hash_map", Span::call_site()),
            MapTy::BTreeMap => Ident::new("btree_map", Span::call_site()),
        }
    }

    fn lib(&self) -> TokenStream {
        match self {
            MapTy::HashMap => quote! { std },
            MapTy::BTreeMap => quote! { prost::alloc },
        }
    }
}

fn fake_scalar(ty: scalar::Ty) -> scalar::Field {
    let kind = scalar::Kind::Plain(scalar::DefaultValue::new(&ty));
    scalar::Field {
        field_ty: Type::Verbatim(ty.rust_type()),
        ty,
        kind,
        tag: 0, // Not used here
        as_msg: None,
        to_msg: None,
        from_msg: None,
        merge_msg: None,
    }
}

#[derive(Clone)]
pub struct Field {
    pub field_value_ty: Type,
    pub map_ty: MapTy,
    pub key_ty: scalar::Ty,
    pub value_ty: ValueTy,
    pub tag: u32,
    pub as_msg: Option<TokenStream>,
    pub to_msg: Option<TokenStream>,
    pub from_msg: Option<TokenStream>,
    pub merge_msg: Option<TokenStream>,
}

impl Field {
    pub fn new(
        field_ty: &Type,
        attrs: &[Meta],
        inferred_tag: Option<u32>,
    ) -> Result<Option<Field>, Error> {
        let mut types = None;
        let mut tag = None;
        let mut as_msg = None;
        let mut to_msg = None;
        let mut from_msg = None;
        let mut merge_msg = None;

        let field_value_ty;
        if let Type::Path(TypePath {
            path: Path { segments, .. },
            ..
        }) = field_ty
        {
            if let Some(PathSegment {
                arguments:
                    PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }),
                ..
            }) = segments.last()
            {
                if let Some(GenericArgument::Type(ty)) = args.last() {
                    field_value_ty = ty.clone();
                } else {
                    return Ok(None);
                }
            } else {
                return Ok(None);
            }
        } else {
            return Ok(None);
        }

        for attr in attrs {
            if let Some(t) = tag_attr(attr)? {
                set_option(&mut tag, t, "duplicate tag attributes")?;
            } else if let Some(a) = as_msg_attr(attr)? {
                set_option(&mut as_msg, a, "duplicate as_msg attributes")?;
            } else if let Some(t) = to_msg_attr(attr)? {
                set_option(&mut to_msg, t, "duplicate to_msg attributes")?;
            } else if let Some(f) = from_msg_attr(attr)? {
                set_option(&mut from_msg, f, "duplicate from_msg attributes")?;
            } else if let Some(m) = merge_msg_attr(attr)? {
                set_option(&mut merge_msg, m, "duplicate merge_msg attributes")?;
            } else if let Some(map_ty) = attr
                .path()
                .get_ident()
                .and_then(|i| MapTy::from_str(&i.to_string()))
            {
                let (k, v): (String, String) = match &*attr {
                    Meta::NameValue(MetaNameValue {
                        lit: Lit::Str(lit), ..
                    }) => {
                        let items = lit.value();
                        let mut items = items.split(',').map(ToString::to_string);
                        let k = items.next().unwrap();
                        let v = match items.next() {
                            Some(k) => k,
                            None => bail!("invalid map attribute: must have key and value types"),
                        };
                        if items.next().is_some() {
                            bail!("invalid map attribute: {:?}", attr);
                        }
                        (k, v)
                    }
                    Meta::List(meta_list) => {
                        // TODO(rustlang/rust#23121): slice pattern matching would make this much nicer.
                        if meta_list.nested.len() != 2 {
                            bail!("invalid map attribute: must contain key and value types");
                        }
                        let k = match &meta_list.nested[0] {
                            NestedMeta::Meta(Meta::Path(k)) if k.get_ident().is_some() => {
                                k.get_ident().unwrap().to_string()
                            }
                            _ => bail!("invalid map attribute: key must be an identifier"),
                        };
                        let v = match &meta_list.nested[1] {
                            NestedMeta::Meta(Meta::Path(v)) if v.get_ident().is_some() => {
                                v.get_ident().unwrap().to_string()
                            }
                            _ => bail!("invalid map attribute: value must be an identifier"),
                        };
                        (k, v)
                    }
                    _ => return Ok(None),
                };
                set_option(
                    &mut types,
                    (map_ty, key_ty_from_str(&k)?, ValueTy::from_str(&v)?),
                    "duplicate map type attribute",
                )?;
            } else {
                return Ok(None);
            }
        }

        ensure!(
            (as_msg.is_none() && to_msg.is_none()) || (from_msg.is_some() || merge_msg.is_some()),
            "missing from_msg or merge_msg attribute",
        );

        ensure!(
            (from_msg.is_none() && merge_msg.is_none()) || (as_msg.is_some() || to_msg.is_some()),
            "missing as_msg or to_msg attribute",
        );

        Ok(match (types, tag.or(inferred_tag)) {
            (Some((map_ty, key_ty, value_ty)), Some(tag)) => {
                if let ValueTy::Scalar(scalar::Ty::Enumeration(..)) = value_ty {
                    ensure!(
                        as_msg.is_none() && to_msg.is_none()
                            && from_msg.is_none() && merge_msg.is_none(),
                        "map fields with enumerations as values cannot have as_msg, to_msg, from_msg, or merge_msg attributes",
                    )
                }

                Some(Field {
                    field_value_ty,
                    map_ty,
                    key_ty,
                    value_ty,
                    tag,
                    as_msg,
                    to_msg,
                    from_msg,
                    merge_msg,
                })
            }
            _ => None,
        })
    }

    pub fn new_oneof(attrs: &[Meta]) -> Result<Option<Field>, Error> {
        if let Some(field) = Field::new(&Type::Verbatim(quote!()), attrs, None)? {
            ensure!(
                field.as_msg.is_none()
                    && field.to_msg.is_none()
                    && field.from_msg.is_none()
                    && field.merge_msg.is_none(),
                "oneof messages cannot have as_msg, to_msg, from_msg, or merge_msg attributes",
            );

            Ok(Some(field))
        } else {
            Ok(None)
        }
    }

    /// Returns a statement which encodes the map field.
    pub fn encode(&self, ident: TokenStream) -> TokenStream {
        let tag = self.tag;
        let key_mod = self.key_ty.module();
        let ke = quote!(::prost::encoding::#key_mod::encode);
        let kl = quote!(::prost::encoding::#key_mod::encoded_len);
        let module = self.map_ty.module();

        let encoding_mod = match &self.value_ty {
            ValueTy::Scalar(scalar::Ty::Enumeration(ty)) => {
                let default = quote!(#ty::default() as i32);
                return quote! {
                    ::prost::encoding::#module::encode_with_default(
                        #ke,
                        #kl,
                        ::prost::encoding::int32::encode,
                        ::prost::encoding::int32::encoded_len,
                        &(#default),
                        #tag,
                        &#ident,
                        buf,
                    );
                };
            }
            ValueTy::Scalar(value_ty) => {
                let val_mod = value_ty.module();
                quote!(::prost::encoding::#val_mod)
            }
            ValueTy::Message => quote!(::prost::encoding::message),
        };

        let (ve, vl) = match (&self.as_msg, &self.to_msg) {
            (Some(as_msg), _) => (
                quote! {
                    |tag, value, buf| #encoding_mod::encode(tag, #as_msg(value), buf)
                },
                quote! {
                    |tag, value| #encoding_mod::encoded_len(tag, #as_msg(value))
                },
            ),
            (None, Some(to_msg)) => (
                quote! {
                    |tag, value, buf| #encoding_mod::encode(tag, &#to_msg(value), buf)
                },
                quote! {
                    |tag, value| #encoding_mod::encoded_len(tag, &#to_msg(value))
                },
            ),
            (None, None) => (
                quote!(#encoding_mod::encode),
                quote!(#encoding_mod::encoded_len),
            ),
        };

        quote! {
            ::prost::encoding::#module::encode(#ke, #kl, #ve, #vl, #tag, &#ident, buf);
        }
    }

    /// Returns an expression which evaluates to the result of merging a decoded key value pair
    /// into the map.
    pub fn merge(&self, ident: TokenStream) -> TokenStream {
        let key_mod = self.key_ty.module();
        let km = quote!(::prost::encoding::#key_mod::merge);
        let module = self.map_ty.module();

        let encoding_mod = match &self.value_ty {
            ValueTy::Scalar(scalar::Ty::Enumeration(ty)) => {
                let default = quote!(#ty::default() as i32);
                return quote! {
                    ::prost::encoding::#module::merge_with_default(
                        #km,
                        ::prost::encoding::int32::merge,
                        #default,
                        &mut #ident,
                        buf,
                        ctx,
                    )
                };
            }
            ValueTy::Scalar(value_ty) => {
                let val_mod = value_ty.module();
                quote!(::prost::encoding::#val_mod)
            }
            ValueTy::Message => quote!(::prost::encoding::message),
        };

        let vm = match (&self.from_msg, &self.merge_msg) {
            (_, Some(merge_msg)) => quote! {
                |wire_type, value, buf, ctx| {
                    let mut msg = Default::default();
                    #encoding_mod::merge(wire_type, &mut msg, buf, ctx)
                        .map(|_| #merge_msg(value, msg))
                }
            },
            (Some(from_msg), None) => quote! {
                |wire_type, value, buf, ctx| {
                    let mut msg = Default::default();
                    #encoding_mod::merge(wire_type, &mut msg, buf, ctx)
                        .map(|_| *value = #from_msg(msg))
                }
            },
            (None, None) => quote!(#encoding_mod::merge),
        };

        quote! {
            ::prost::encoding::#module::merge(#km, #vm, &mut #ident, buf, ctx)
        }
    }

    /// Returns an expression which evaluates to the encoded length of the map.
    pub fn encoded_len(&self, ident: TokenStream) -> TokenStream {
        let tag = self.tag;
        let key_mod = self.key_ty.module();
        let kl = quote!(::prost::encoding::#key_mod::encoded_len);
        let module = self.map_ty.module();
        let encoding_mod = match &self.value_ty {
            ValueTy::Scalar(scalar::Ty::Enumeration(ty)) => {
                let default = quote!(#ty::default() as i32);
                return quote! {
                    ::prost::encoding::#module::encoded_len_with_default(
                        #kl,
                        ::prost::encoding::int32::encoded_len,
                        &(#default),
                        #tag,
                        &#ident,
                    )
                };
            }
            ValueTy::Scalar(value_ty) => {
                let val_mod = value_ty.module();
                quote!(::prost::encoding::#val_mod)
            }
            ValueTy::Message => quote!(::prost::encoding::message),
        };

        let vl = match (&self.as_msg, &self.to_msg) {
            (Some(as_msg), _) => quote! {
                |tag, value| #encoding_mod::encoded_len(tag, #as_msg(value))
            },
            (None, Some(to_msg)) => quote! {
                |tag, value| #encoding_mod::encoded_len(tag, &#to_msg(value))
            },
            (None, None) => quote!(#encoding_mod::encoded_len),
        };

        quote! {
            ::prost::encoding::#module::encoded_len(#kl, #vl, #tag, &#ident)
        }
    }

    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        quote!(#ident.clear())
    }

    /// Returns methods to embed in the message.
    pub fn methods(&self, ident: &Ident) -> Option<TokenStream> {
        if let ValueTy::Scalar(scalar::Ty::Enumeration(ty)) = &self.value_ty {
            let key_ty = self.key_ty.rust_type();
            let key_ref_ty = self.key_ty.rust_ref_type();

            let get = Ident::new(&format!("get_{}", ident), Span::call_site());
            let insert = Ident::new(&format!("insert_{}", ident), Span::call_site());
            let take_ref = if self.key_ty.is_numeric() {
                quote!(&)
            } else {
                quote!()
            };

            let get_doc = format!(
                "Returns the enum value for the corresponding key in `{}`, \
                 or `None` if the entry does not exist or it is not a valid enum value.",
                ident,
            );
            let insert_doc = format!("Inserts a key value pair into `{}`.", ident);
            Some(quote! {
                #[doc=#get_doc]
                pub fn #get(&self, key: #key_ref_ty) -> ::core::option::Option<#ty> {
                    self.#ident.get(#take_ref key).cloned().and_then(#ty::from_i32)
                }
                #[doc=#insert_doc]
                pub fn #insert(&mut self, key: #key_ty, value: #ty) -> ::core::option::Option<#ty> {
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
    pub fn debug(&self, wrapper_name: TokenStream) -> TokenStream {
        let type_name = match self.map_ty {
            MapTy::HashMap => Ident::new("HashMap", Span::call_site()),
            MapTy::BTreeMap => Ident::new("BTreeMap", Span::call_site()),
        };

        // A fake field for generating the debug wrapper
        let key_wrapper = fake_scalar(self.key_ty.clone()).debug(quote!(KeyWrapper));
        let key = self.key_ty.rust_type();
        let libname = self.map_ty.lib();
        let field_value_ty = &self.field_value_ty;

        match (&self.as_msg, &self.to_msg) {
            (Some(msg_fn), _) | (None, Some(msg_fn)) => quote! {
                struct #wrapper_name<'a>(&'a ::#libname::collections::#type_name<#key, #field_value_ty>);
                impl<'a> ::core::fmt::Debug for #wrapper_name<'a> {
                    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                        #key_wrapper
                        let mut builder = f.debug_map();
                        for (k, v) in self.0 {
                            builder.entry(&KeyWrapper(k), &#msg_fn(v));
                        }
                        builder.finish()
                    }
                }
            },
            (None, None) => match &self.value_ty {
                ValueTy::Scalar(ty) => {
                    let value = ty.rust_type();
                    let value_wrapper = fake_scalar(ty.clone()).debug(quote!(ValueWrapper));

                    quote! {
                        struct #wrapper_name<'a>(&'a ::#libname::collections::#type_name::<#key, #value>);
                        impl<'a> ::core::fmt::Debug for #wrapper_name<'a> {
                            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                                #key_wrapper
                                #value_wrapper
                                let mut builder = f.debug_map();
                                for (k, v) in self.0 {
                                    builder.entry(&KeyWrapper(k), &ValueWrapper(v));
                                }
                                builder.finish()
                            }
                        }
                    }
                }
                ValueTy::Message => quote! {
                    struct #wrapper_name<'a, V: 'a>(&'a ::#libname::collections::#type_name<#key, V>);
                    impl<'a, V> ::core::fmt::Debug for #wrapper_name<'a, V>
                    where
                        V: ::core::fmt::Debug + 'a,
                    {
                        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                            #key_wrapper
                            let mut builder = f.debug_map();
                            for (k, v) in self.0 {
                                builder.entry(&KeyWrapper(k), &v);
                            }
                            builder.finish()
                        }
                    }
                },
            },
        }
    }
}

fn key_ty_from_str(s: &str) -> Result<scalar::Ty, Error> {
    let ty = scalar::Ty::from_str(s)?;
    match ty {
        scalar::Ty::Int32
        | scalar::Ty::Int64
        | scalar::Ty::Uint32
        | scalar::Ty::Uint64
        | scalar::Ty::Sint32
        | scalar::Ty::Sint64
        | scalar::Ty::Fixed32
        | scalar::Ty::Fixed64
        | scalar::Ty::Sfixed32
        | scalar::Ty::Sfixed64
        | scalar::Ty::Bool
        | scalar::Ty::String => Ok(ty),
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
}
