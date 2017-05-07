// The `quote!` macro requires deep recursion.
#![recursion_limit = "1024"]

extern crate itertools;
extern crate proc_macro;
extern crate syn;

#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate quote;

use std::ascii::AsciiExt;
use std::fmt;
use std::slice;
use std::str::{self, FromStr};

use itertools::Itertools;
use proc_macro::TokenStream;
use quote::{ToTokens, Tokens};

// Proc-macro crates can't export anything, so error chain definitions go in a private module.
mod error {
    error_chain!();
}
use error::*;

fn concat_tokens(mut sum: Tokens, rest: Tokens) -> Tokens {
    sum.append(rest.as_str());
    sum
}

/// A protobuf field type.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FieldKind {
    Double,
    Float,
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
    Bytes,
    Enum,
    Message,
}

impl FieldKind {
    fn as_str(&self) -> &'static str {
        match *self {
            FieldKind::Double => "double",
            FieldKind::Float => "double",
            FieldKind::Int32 => "int32",
            FieldKind::Int64 => "int64",
            FieldKind::Uint32 => "uint32",
            FieldKind::Uint64 => "uint64",
            FieldKind::Sint32 => "sint32",
            FieldKind::Sint64 => "sint64",
            FieldKind::Fixed32 => "fixed32",
            FieldKind::Fixed64 => "fixed64",
            FieldKind::Sfixed32 => "sfixed32",
            FieldKind::Sfixed64 => "sfixed64",
            FieldKind::Bool => "bool",
            FieldKind::String => "string",
            FieldKind::Bytes => "bytes",
            FieldKind::Enum => "enum",
            FieldKind::Message => "message",
        }
    }

    fn variants() -> slice::Iter<'static, FieldKind> {
        const VARIANTS: &'static [FieldKind] = &[
            FieldKind::Double,
            FieldKind::Float,
            FieldKind::Int32,
            FieldKind::Int64,
            FieldKind::Uint32,
            FieldKind::Uint64,
            FieldKind::Sint32,
            FieldKind::Sint64,
            FieldKind::Fixed32,
            FieldKind::Fixed64,
            FieldKind::Sfixed32,
            FieldKind::Sfixed64,
            FieldKind::Bool,
            FieldKind::String,
            FieldKind::Bytes,
            FieldKind::Enum,
            FieldKind::Message,
        ];
        VARIANTS.iter()
    }

    /// Parses a string into a field type.
    /// If the string doesn't match a field type, `None` is returned.
    fn from_str(s: &str) -> Option<FieldKind> {
        for &kind in FieldKind::variants() {
            if s.eq_ignore_ascii_case(kind.as_str()) {
                return Some(kind);
            }
        }
        None
    }
}

impl fmt::Debug for FieldKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for FieldKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FieldLabel {
    /// An optional field.
    Optional,
    /// A required field.
    Required,
    /// A repeated field.
    Repeated,
}

impl FieldLabel {
    fn as_str(&self) -> &'static str {
        match *self {
            FieldLabel::Optional => "optional",
            FieldLabel::Required => "required",
            FieldLabel::Repeated => "repeated",
        }
    }

    fn variants() -> slice::Iter<'static, FieldLabel> {
        const VARIANTS: &'static [FieldLabel] = &[
            FieldLabel::Optional,
            FieldLabel::Required,
            FieldLabel::Repeated,
        ];
        VARIANTS.iter()
    }

    /// Parses a string into a field label.
    /// If the string doesn't match a field label, `None` is returned.
    fn from_str(s: &str) -> Option<FieldLabel> {
        for &label in FieldLabel::variants() {
            if s.eq_ignore_ascii_case(label.as_str()) {
                return Some(label);
            }
        }
        None
    }
}

impl fmt::Debug for FieldLabel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for FieldLabel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

enum Field {
    /// An ordinary field.
    Field {
        kind: FieldKind,
        label: Option<FieldLabel>,
        tag: u32,
        default: Option<syn::Lit>,
    },
    /// A map field.
    Map {
        key_kind: FieldKind,
        value_kind: FieldKind,
        tag: u32,
    },
    /// A oneof field.
    Oneof {
        tags: Vec<u32>,
    },
}

impl Field {

    /// Converts an iterator of meta items into a field descriptor.
    ///
    /// If the meta items are invalid, an error will be returned.
    /// If the field should be ignored, `None` is returned.
    fn from_attrs(attrs: &[syn::Attribute]) -> Result<Option<Field>> {

        fn lit_to_field_kind(lit: &syn::Lit) -> Result<FieldKind> {
            let s = if let syn::Lit::Str(ref s, _) = *lit {
                s
            } else {
                bail!("invalid type: {:?}", lit);
            };

            FieldKind::from_str(s).map(|kind| Ok(kind))
                                  .unwrap_or_else(|| bail!("unknown type: {}", s))
        }

        fn lit_to_tag(lit: &syn::Lit) -> Result<u32> {
            match *lit {
                syn::Lit::Str(ref s, _) => s.parse::<u32>().map_err(|err| Error::from(err.to_string())),
                syn::Lit::Int(i, _) => Ok(i as u32),
                _ => bail!("{:?}", lit),
            }
        }

        fn lit_to_tags(lit: &syn::Lit) -> Result<Vec<u32>> {
            match *lit {
                syn::Lit::Str(ref s, _) => {
                    s.split(",")
                     .map(|s| s.trim().parse::<u32>().map_err(|err| Error::from(err.to_string())))
                     .collect()
                },
                _ => bail!("{:?}", lit),
            }
        }

        // Ordinary field options.
        let mut kind = None;
        let mut label = None;
        let mut packed = None;
        let mut tag = None;
        let mut default = None;

        // Map field options.
        let mut map = false;
        let mut key_kind = None;
        let mut value_kind = None;

        // Oneof field options.
        let mut oneof = false;
        let mut tags = None;

        // Get the items belonging to the 'proto' list attribute (e.g. #[proto(foo, bar="baz")]).
        let proto_attrs = attrs.iter().flat_map(|attr| {
            match attr.value {
                syn::MetaItem::List(ref ident, ref items) if ident == "proto" => items.into_iter(),
                _ => [].into_iter(),
            }
        });

        // Parse the field attributes into the corresponding option fields.
        for meta_item in proto_attrs {
            match *meta_item {
                syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref word)) => {
                    let word = word.as_ref();
                    if word.eq_ignore_ascii_case("ignore") {
                        return Ok(None);
                    } else if word.eq_ignore_ascii_case("map") {
                        map = true;
                    } else if word.eq_ignore_ascii_case("oneof") {
                        oneof = true;
                    } else if let Some(field_kind) = FieldKind::from_str(word) {
                        if let Some(existing_kind) = kind {
                            bail!("duplicate type attributes: {} and {}", existing_kind, field_kind);
                        }
                        kind = Some(field_kind);
                    } else if let Some(field_label) = FieldLabel::from_str(word) {
                        if let Some(existing_label) = label {
                            bail!("duplicate label attributes: {:?} and {:?}", existing_label, field_label);
                        }
                        label = Some(field_label);

                    } else {
                        bail!("unknown attribute: {}", word);
                    }
                },
                syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, ref value)) => {
                    let name = name.as_ref();
                    if name.eq_ignore_ascii_case("tag") {
                        if tag.is_some() {
                            bail!("duplicate tag attributes");
                        }
                        let field_tag = lit_to_tag(&value).chain_err(|| "invalid tag attribute")?;
                        tag = Some(field_tag);
                    } else if name.eq_ignore_ascii_case("tags") {
                        if tags.is_some() {
                            bail!("duplicate tags attributes");
                        }
                        let field_tags = lit_to_tags(&value).chain_err(|| "invalid tags attribute")?;
                        tags = Some(field_tags);
                    } else if name.eq_ignore_ascii_case("key") {
                        let field_key_kind = lit_to_field_kind(&value).chain_err(|| "invalid map key type attribute")?;
                        if let Some(existing_key_kind) = key_kind {
                            bail!("duplicate map key type attributes: {} and {}",
                                  existing_key_kind, field_key_kind);
                        }
                        key_kind = Some(field_key_kind);
                    } else if name.eq_ignore_ascii_case("value") {
                        let field_value_kind = lit_to_field_kind(&value).chain_err(|| "invalid map value type attribute")?;
                        if let Some(existing_value_kind) = value_kind {
                            bail!("duplicate map value type attributes: {} and {}",
                                  existing_value_kind, field_value_kind);
                        }
                        value_kind = Some(field_value_kind);
                    } else if name.eq_ignore_ascii_case("packed") {
                        if packed.is_some() {
                            bail!("duplicate packed attributes");
                        }
                        let field_packed = lit_to_bool(&value).chain_err(|| "illegal packed attribute")?;
                        packed = Some(field_packed);
                    } else if name.eq_ignore_ascii_case("default") {
                        if default.is_some() {
                            bail!("duplicate default attributes");
                        }
                        default = Some(value);
                    }
                },
                syn::NestedMetaItem::Literal(lit) => bail!("invalid field attribute: {:?}", lit),
                syn::NestedMetaItem::MetaItem(syn::MetaItem::List(ref ident, _)) => bail!("invalid field attributes: {}", ident),
            }
        }

        // Check that either the field is an ordinary type, a map, or a oneof.
        match (kind, map, oneof) {
            (Some(_), false, false) | (None, true, false) | (None, false, true) => (),
            (Some(kind), true, _) => bail!("field may not be a {} and a map", kind),
            (Some(kind), _, true) => bail!("field may not be a {} and a oneof", kind),
            (_, true, true) => bail!("field may not be a map and a oneof"),
            (None, false, false) => bail!("field must have a type attribute"),
        }

        let field = if let Some(kind) = kind {
            if key_kind.is_some() { bail!("invalid key type attribute for {} field", kind); }
            if value_kind.is_some() { bail!("invalid value type attribute for {} field", kind); }
            if tags.is_some() { bail!("invalid tags attribute for {} field", kind); }

            let tag = match tag {
                Some(tag) => tag,
                None => bail!("{} field must have a tag attribute", kind),
            };

            Field::Field {
                kind: kind,
                label: label,
                tag: tag,
                default: default.cloned(),
            }
        } else if map {
            if let Some(label) = label { bail!("invalid {} attribute for map field", label); }
            if packed.is_some() { bail!("invalid packed attribute for map field"); }
            if default.is_some() { bail!("invalid default attribute for map field"); }
            if tags.is_some() { bail!("invalid tags attribute for oneof field"); }

            let tag = match tag {
                Some(tag) => tag,
                None => bail!("map field must have a tag attribute"),
            };

            let key_kind = match key_kind {
                Some(key_kind) => key_kind,
                None => bail!("map field must have a key type attribute"),
            };

            let value_kind = match value_kind {
                Some(value_kind) => value_kind,
                None => bail!("map field must have a value type attribute"),
            };

            Field::Map {
                key_kind: key_kind,
                value_kind: value_kind,
                tag: tag,
            }
        } else {
            assert!(oneof);
            if let Some(label) = label { bail!("invalid {} attribute for oneof field", label); }
            if packed.is_some() { bail!("invalid packed attribute for oneof field"); }
            if default.is_some() { bail!("invalid default attribute for oneof field"); }
            if tag.is_some() { bail!("invalid tag attribute for oneof field"); }
            if key_kind.is_some() { bail!("invalid key type attribute for oneof field"); }
            if value_kind.is_some() { bail!("invalid value type attribute for oneof field"); }

            let tags = match tags {
                Some(tags) => tags,
                None => bail!("oneof field must have a tags attribute"),
            };

            Field::Oneof {
                tags: tags,
            }
        };

        Ok(Some(field))
    }

    fn tags(&self) -> &[u32] {
        match *self {
            Field::Field { tag, .. } => &[tag],
            Field::Map { tag, .. } => &[tag],
            Field::Oneof { ref tags, .. } => tags,
        }
    }

    fn encode(&self) -> Tokens {
        match *self {
            Field::Field { kind, .. } => {

            },
            Field::Map { .. } => {
            },
            Field::Oneof { .. } => {
            },
        }
    }
}

fn try_message(input: TokenStream) -> Result<TokenStream> {
    let syn::DeriveInput { ident, generics, body, .. } =
        syn::parse_derive_input(&input.to_string()).expect("unable to parse message type");

    if !generics.lifetimes.is_empty() ||
       !generics.ty_params.is_empty() ||
       !generics.where_clause.predicates.is_empty() {
        panic!("Message may not be derived for generic type");
    }

    let fields = match body {
        syn::Body::Struct(syn::VariantData::Struct(fields)) => fields,
        syn::Body::Struct(syn::VariantData::Tuple(fields)) => fields,
        syn::Body::Struct(syn::VariantData::Unit) => Vec::new(),
        syn::Body::Enum(..) => panic!("Message can not be derived for an enum"),
    };

    let fields: Vec<Field> = fields.into_iter()
                                   .enumerate()
                                   .flat_map(|(idx, field)| {
                                       match Field::from_attrs(&field.attrs) {
                                           Ok(Some(field)) => Some(Ok(field)),
                                           Ok(None) => None,
                                           Err(err) => Some(Err(err).chain_err(|| {
                                               match field.ident {
                                                   Some(ref field_ident) =>
                                                       format!("invalid message field {}.{}",
                                                               ident, field_ident),
                                                   None => format!("invalid message field {}.{}",
                                                                   ident, idx),
                                               }
                                           })),
                                       }
                                   })
                                   .collect()?;

    let mut tags = fields.iter().flat_map(|field| field.tags()).collect::<Vec<_>>();
    let num_tags = tags.len();
    tags.sort();
    tags.dedup();
    if tags.len() != num_tags {
        bail!("message {} has fields with duplicate tags", ident);
    }

    let dummy_const = syn::Ident::new(format!("_IMPL_MESSAGE_FOR_{}", ident));
    let encoded_len = encoded_len(&fields);

    let encode = fields.iter().map(|field| {
        let kind = &field.kind;
        let tag = field.tags[0];
        let field = &field.ident;
        quote! { _proto::field::Field::<#kind>::encode(&self.#field, #tag, buf); }
    }).fold(Tokens::new(), concat_tokens);

    let merge = fields.iter().map(|field| {
        let tags = field.tags.iter().map(|tag| quote!(#tag)).intersperse(quote!(|)).fold(Tokens::new(), concat_tokens);
        let kind = &field.kind;
        let field = &field.ident;
        quote!{ #tags => _proto::field::Field::<#kind>::merge(&mut self.#field, tag, wire_type, buf)
                                                        .map_err(|error| {
                                                            ::std::io::Error::new(
                                                                error.kind(),
                                                                format!(concat!("failed to decode field ", stringify!(#ident),
                                                                                ".", stringify!(#field), ": {}"),
                                                                        error))
                                                        })?, }
    }).fold(Tokens::new(), concat_tokens);

    let default = default(&fields);

    let expanded = quote! {
        #[allow(
            non_upper_case_globals,
            unused_attributes,
            unused_imports,
            unused_qualifications,
            unused_variables
        )]
        const #dummy_const: () = {

            extern crate proto as _proto;
            extern crate bytes as _bytes;

            #[automatically_derived]
            impl _proto::Message for #ident {
                #[inline]
                fn encode<B>(&self, buf: &mut B) -> ::std::io::Result<()> where B: _bytes::BufMut {
                    #encode
                    Ok(())
                }

                #[inline]
                fn merge<B>(&mut self, buf: &mut _bytes::Take<B>) -> ::std::io::Result<()> where B: _bytes::Buf {
                    while _bytes::Buf::has_remaining(buf) {
                        let (tag, wire_type) = _proto::encoding::decode_key(buf)?;
                        match tag {
                            #merge
                            _ => _proto::encoding::skip_field(wire_type, buf)?,
                        }
                    }
                    Ok(())
                }

                #[inline]
                fn encoded_len(&self) -> usize {
                    #encoded_len
                }
            }

            #[automatically_derived]
            impl Default for #ident {
                fn default() -> #ident {
                    #ident {
                        #default
                    }
                }
            }
        };
    };

    expanded.parse().unwrap()

}

#[proc_macro_derive(Message, attributes(proto))]
pub fn message(input: TokenStream) -> TokenStream {
    try_message(input).unwrap()
}

fn encoded_len(fields: &[Field]) -> Tokens {
    fields.iter().map(|field| {
        let kind = &field.kind;
        let ident = &field.ident;
        let tag = field.tags[0];
        match field.default {
            Some(ref tokens) => quote! {
                if self.#ident == #tokens {
                    0
                } else {
                    _proto::field::Field::<#kind>::encoded_len(&self.#ident, #tag)
                }
            },
            None => quote! {
                if self.#ident == ::std::default::Default::default() {
                    0
                } else {
                    _proto::field::Field::<#kind>::encoded_len(&self.#ident, #tag)
                }
            },
        }
    })
    .fold(quote!(0), |mut sum, expr| {
        sum.append("+");
        sum.append(expr.as_str());
        sum
    })
}

fn default(fields: &[Field]) -> Tokens {
    fields.iter().map(|field| {
        let ident = &field.ident;
        match field.default {
            Some(ref default) => {
                let lit = default_value(default.clone());
                quote!(#ident: #lit,)
            },
            // Total hack: if a oneof, default to None (we always wrap oneofs in a None).
            // This really should be checking the type, but we don't have a typesafe type, its just
            // tokens.
            None if field.tags.len() > 1 => quote!(#ident: None),
            None => quote!(#ident: Default::default(),),
        }
    })
    .fold(Tokens::new(), concat_tokens)
}

fn default_value(lit: syn::Lit) -> Tokens {
    match lit {
        syn::Lit::Str(s, _) => {
            let mut tokens = Tokens::new();
            for tt in syn::parse_token_trees(&s).expect(&format!("unable to parse default literal value: {}", s)) {
                tt.to_tokens(&mut tokens);
            }
            quote!(#tokens)
        },
        other => quote!(#other),
    }
}

/*
#[proc_macro_derive(Enumeration, attributes(proto))]
pub fn enumeration(input: TokenStream) -> TokenStream {
    let syn::DeriveInput { ident, generics, attrs, body, .. } =
        syn::parse_derive_input(&input.to_string()).expect("unable to parse enumeration type");

    if !generics.lifetimes.is_empty() ||
       !generics.ty_params.is_empty() ||
       !generics.where_clause.predicates.is_empty() {
        panic!("Enumeration may not be derived for generic type");
    }

    let variants = match body {
        syn::Body::Struct(..) => panic!("Enumeration can not be derived for a struct"),
        syn::Body::Enum(variants) => variants,
    };

    let variants = variants.into_iter().map(|syn::Variant { ident: variant, data, discriminant, .. }| {
        if let syn::VariantData::Unit = data {
            if let Some(discriminant) = discriminant {
                (variant, discriminant)
            } else {
                panic!("Enumeration variants must have a discriminant value: {}::{}", ident, variant);
            }
        } else {
            panic!("Enumeration variants may not have fields: {}::{}", ident, variant);
        }
    }).collect::<Vec<_>>();

    if variants.is_empty() {
        panic!("Enumeration must have at least one variant: {}", ident);
    }

    let repr = attrs.into_iter()
                    .filter_map(|mut attr: syn::Attribute| match attr.value {
                        syn::MetaItem::List(ref attr, ref mut repr) if attr == "repr" => repr.pop(),
                        _ => None,
                    })
                    .last()
                    .map_or_else(|| quote!(i64), |repr| quote!(#repr));

    let default = variants[0].0.clone();

    let dummy_const = syn::Ident::new(format!("_IMPL_ENUMERATION_FOR_{}", ident));
    let is_valid = variants.iter()
                           .map(|&(_, ref value)| quote!(#value => true,))
                           .fold(Tokens::new(), concat_tokens);
    let from = variants.iter()
                       .map(|&(ref variant, ref value)| quote!(#value => #ident::#variant,))
                       .fold(Tokens::new(), concat_tokens);
    let from_str = variants.iter()
                           .map(|&(ref variant, _)| quote!(stringify!(#variant) => Ok(#ident::#variant),))
                           .fold(Tokens::new(), concat_tokens);

    let expanded = quote! {
        #[allow(non_upper_case_globals, unused_attributes, unused_qualifications)]
        const #dummy_const: () = {
            extern crate bytes as _bytes;
            extern crate proto as _proto;

            impl #ident {
                fn is_valid(value: #repr) -> bool {
                    match value {
                        #is_valid
                        _ => false,
                    }
                }
            }

            #[automatically_derived]
            impl _proto::field::Field<_proto::field::Enumeration> for #ident {

                #[inline]
                fn encode<B>(&self, tag: u32, buf: &mut B) where B: _bytes::BufMut {
                    _proto::encoding::encode_key(tag, _proto::encoding::WireType::Varint, buf);
                    _proto::encoding::encode_varint(*self as u64, buf);
                }

                #[inline]
                fn merge<B>(&mut self, _tag: u32, wire_type: _proto::encoding::WireType, buf: &mut _bytes::Take<B>) -> ::std::io::Result<()> where B: _bytes::Buf {
                    _proto::encoding::check_wire_type(_proto::encoding::WireType::Varint, wire_type)?;
                    *self = #ident::from(_proto::encoding::decode_varint(buf)? as #repr);
                    Ok(())
                }

                #[inline]
                fn encoded_len(&self, tag: u32) -> usize {
                    _proto::encoding::key_len(tag) + _proto::encoding::encoded_len_varint(*self as u64)
                }
            }

            impl _proto::field::Type<_proto::field::Enumeration> for #ident {}

            #[automatically_derived]
            impl Default for #ident {
                fn default() -> #ident {
                    #ident::#default
                }
            }

            #[automatically_derived]
            impl From<#repr> for #ident {
                fn from(value: #repr) -> #ident {
                    match value {
                        #from
                        _ => #ident::#default,
                    }
                }
            }

            #[automatically_derived]
            impl ::std::str::FromStr for #ident {
                type Err = String;
                fn from_str(s: &str) -> ::std::result::Result<#ident, String> {
                    match s {
                        #from_str
                        _ => Err(format!(concat!("unknown ", stringify!(#ident), " variant: {}"), s)),
                    }
                }
            }
        };
    };

    expanded.parse().unwrap()
}

#[proc_macro_derive(Oneof, attributes(proto))]
pub fn oneof(input: TokenStream) -> TokenStream {
    let syn::DeriveInput { ident, generics, body, .. } =
        syn::parse_derive_input(&input.to_string()).expect("unable to parse message type");

    if !generics.lifetimes.is_empty() ||
       !generics.ty_params.is_empty() ||
       !generics.where_clause.predicates.is_empty() {
        panic!("Oneof may not be derived for generic type");
    }

    let variants = match body {
        syn::Body::Enum(variants) => variants,
        syn::Body::Struct(..) => panic!("Oneof can not be derived for a struct"),
    };

    // Map the variants into 'fields'.
    let fields = variants.into_iter().map(|variant| {
        let ident = variant.ident;
        let attrs = variant.attrs;
        if let syn::VariantData::Tuple(mut fields) = variant.data {
            if fields.len() != 1 {
                panic!("Oneof enum must contain only tuple variants with a single field");
            }
            let field = fields.pop().unwrap();
            Field::extract(syn::Field {
                ident: Some(ident),
                vis: field.vis,
                attrs: attrs,
                ty: field.ty,
            }).expect("Oneof fields may not be ignored")
        } else {
            panic!("Oneof enum must contain only tuple variants with a single field");
        }
    }).collect::<Vec<_>>();

    let mut tags = fields.iter().flat_map(|field| {
        if field.tags.len() > 1 {
            panic!("proto oneof variants may only have a single tag: {}.{}",
                   ident, field.ident);
        }
        &field.tags
    }).collect::<Vec<_>>();
    tags.sort();
    tags.dedup();
    if tags.len() != fields.len() {
        panic!("proto oneof variants may not have duplicate tags: {}", ident);
    }

    let dummy_const = syn::Ident::new(format!("_IMPL_ONEOF_FOR_{}", ident));

    let encode = fields.iter().map(|field| {
        let kind = &field.kind;
        let name = &field.ident;
        let tag = field.tags[0];
        quote! { #ident::#name(ref value) => _proto::field::Field::<#kind>::encode(value, #tag, buf), }
    }).fold(Tokens::new(), concat_tokens);

    let merge = fields.iter().map(|field| {
        let kind = &field.kind;
        let name = &field.ident;
        let tag = field.tags[0];
        quote! { #tag => {
            let mut value = ::std::default::Default::default();
            _proto::field::Field::<#kind>::merge(&mut value, tag, wire_type, buf)?;
            #ident::#name(value)
        },
        }
    }).fold(Tokens::new(), concat_tokens);

    let encoded_len = fields.iter().map(|field| {
        let kind = &field.kind;
        let name = &field.ident;
        let tag = field.tags[0];
        quote! { #ident::#name(ref value) => _proto::field::Field::<#kind>::encoded_len(value, #tag), }
    }).fold(Tokens::new(), concat_tokens);

    let default = {
        let field_ident = &fields[0].ident;
        quote!(#ident::#field_ident(::std::default::Default::default()))
    };

    let expanded = quote! {
        #[allow(
            non_upper_case_globals,
            unused_attributes,
            unused_imports,
            unused_qualifications,
            unused_variables
        )]
        const #dummy_const: () = {
            extern crate bytes as _bytes;
            extern crate proto as _proto;

            #[automatically_derived]
            impl _proto::field::Field<_proto::field::Oneof> for #ident {
                #[inline]
                fn encode<B>(&self, tag: u32, buf: &mut B) where B: _bytes::BufMut {
                    match *self {
                        #encode
                    }
                }

                #[inline]
                fn merge<B>(&mut self, tag: u32, wire_type: _proto::encoding::WireType, buf: &mut _bytes::Take<B>) -> ::std::io::Result<()> where B: _bytes::Buf {
                    *self = match tag {
                        #merge
                        _ => panic!("unknown tag: {}", tag),
                    };
                    ::std::result::Result::Ok(())
                }

                #[inline]
                fn encoded_len(&self, tag: u32) -> usize {
                    match *self {
                        #encoded_len
                    }
                }
            }

            impl ::std::default::Default for #ident {
                fn default() -> #ident {
                    #default
                }
            }

            impl _proto::field::Type<_proto::field::Oneof> for #ident {}
        };
    };

    expanded.parse().unwrap()
}
*/

/// Parses a literal value into a bool.
fn lit_to_bool(lit: &syn::Lit) -> Result<bool> {
    match *lit {
        syn::Lit::Bool(b) => Ok(b),
        syn::Lit::Str(ref s, _) => s.parse::<bool>().map_err(|e| Error::from(e.to_string())),
        _ => bail!("invalid literal value: {:?}", lit),
    }
}
