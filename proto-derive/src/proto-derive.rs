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
use std::str;

use itertools::Itertools;
use proc_macro::TokenStream;
use quote::Tokens;

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
enum ScalarType {
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
}

impl ScalarType {
    fn as_str(&self) -> &'static str {
        match *self {
            ScalarType::Double => "double",
            ScalarType::Float => "double",
            ScalarType::Int32 => "int32",
            ScalarType::Int64 => "int64",
            ScalarType::Uint32 => "uint32",
            ScalarType::Uint64 => "uint64",
            ScalarType::Sint32 => "sint32",
            ScalarType::Sint64 => "sint64",
            ScalarType::Fixed32 => "fixed32",
            ScalarType::Fixed64 => "fixed64",
            ScalarType::Sfixed32 => "sfixed32",
            ScalarType::Sfixed64 => "sfixed64",
            ScalarType::Bool => "bool",
            ScalarType::String => "string",
            ScalarType::Bytes => "bytes",
            ScalarType::Enum => "enum",
        }
    }

    fn variants() -> slice::Iter<'static, ScalarType> {
        const VARIANTS: &'static [ScalarType] = &[
            ScalarType::Double,
            ScalarType::Float,
            ScalarType::Int32,
            ScalarType::Int64,
            ScalarType::Uint32,
            ScalarType::Uint64,
            ScalarType::Sint32,
            ScalarType::Sint64,
            ScalarType::Fixed32,
            ScalarType::Fixed64,
            ScalarType::Sfixed32,
            ScalarType::Sfixed64,
            ScalarType::Bool,
            ScalarType::String,
            ScalarType::Bytes,
            ScalarType::Enum,
        ];
        VARIANTS.iter()
    }

    /// Parses a string into a field type.
    /// If the string doesn't match a field type, `None` is returned.
    fn from_str(s: &str) -> Option<ScalarType> {
        for &kind in ScalarType::variants() {
            if s.eq_ignore_ascii_case(kind.as_str()) {
                return Some(kind);
            }
        }
        None
    }

    fn encode(&self, ident: &syn::Ident) -> Tokens {
        match *self {
            ScalarType::Double => quote!(_proto::encoding::encode_double(#ident, buf)),
            ScalarType::Float => quote!(_proto::encoding::encode_float(#ident, buf)),
            ScalarType::Int32 => quote!(_proto::encoding::encode_int32(#ident, buf)),
            ScalarType::Int64 => quote!(_proto::encoding::encode_int64(#ident, buf)),
            ScalarType::Uint32 => quote!(_proto::encoding::encode_uint32(#ident, buf)),
            ScalarType::Uint64 => quote!(_proto::encoding::encode_uint64(#ident, buf)),
            ScalarType::Sint32 => quote!(_proto::encoding::encode_sint32(#ident, buf)),
            ScalarType::Sint64 => quote!(_proto::encoding::encode_sint64(#ident, buf)),
            ScalarType::Fixed32 => quote!(_proto::encoding::encode_fixed32(#ident, buf)),
            ScalarType::Fixed64 => quote!(_proto::encoding::encode_fixed64(#ident, buf)),
            ScalarType::Sfixed32 => quote!(_proto::encoding::encode_sfixed32(#ident, buf)),
            ScalarType::Sfixed64 => quote!(_proto::encoding::encode_sfixed64(#ident, buf)),
            ScalarType::Bool => quote!(_proto::encoding::encode_bool(#ident, buf)),
            ScalarType::String => quote!(_proto::encoding::encode_string(&#ident[..], buf)),
            ScalarType::Bytes => quote!(_proto::encoding::encode_bytes(&#ident[..], buf)),
            ScalarType::Enum => quote!(_proto::encoding::encode_int32(#ident as i32, buf)),
        }
    }

    fn wire_type(&self) -> Tokens {
        match *self {
            ScalarType::Float
                | ScalarType::Fixed32
                | ScalarType::Sfixed32 => quote!(_proto::encoding::WireType::ThirtyTwoBit),
            ScalarType::Double
                | ScalarType::Fixed64
                | ScalarType::Sfixed64 => quote!(_proto::encoding::WireType::SixtyFourBit),
            ScalarType::Int32
                | ScalarType::Int64
                | ScalarType::Uint32
                | ScalarType::Uint64
                | ScalarType::Sint32
                | ScalarType::Sint64
                | ScalarType::Bool
                | ScalarType::Enum => quote!(_proto::encoding::WireType::Varint),
            ScalarType::String
                | ScalarType::Bytes => quote!(_proto::encoding::WireType::LengthDelimited),
        }
    }
}

impl fmt::Debug for ScalarType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for ScalarType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Label {
    /// An optional field.
    Optional,
    /// A required field.
    Required,
    /// A repeated field.
    Repeated,
}

impl Label {
    fn as_str(&self) -> &'static str {
        match *self {
            Label::Optional => "optional",
            Label::Required => "required",
            Label::Repeated => "repeated",
        }
    }

    fn variants() -> slice::Iter<'static, Label> {
        const VARIANTS: &'static [Label] = &[
            Label::Optional,
            Label::Required,
            Label::Repeated,
        ];
        VARIANTS.iter()
    }

    /// Parses a string into a field label.
    /// If the string doesn't match a field label, `None` is returned.
    fn from_str(s: &str) -> Option<Label> {
        for &label in Label::variants() {
            if s.eq_ignore_ascii_case(label.as_str()) {
                return Some(label);
            }
        }
        None
    }
}

impl fmt::Debug for Label {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

fn encode_scalar_field(ident: &syn::Ident, ty: ScalarType, tag: u32, default: Option<&syn::Lit>) -> Tokens {
    let wire_type = ty.wire_type();
    let encode_key = quote!(_proto::encoding::encode_key(#tag, #wire_type, buf));
    let encode = ty.encode(&syn::Ident::new(format!("self.{}", ident)));

    match default {
        Some(default) => {
            quote! {
                #encode_key;
                if self.#ident != #default {
                    #encode;
                }
            }
        },
        None => {
            quote! {
                #encode_key;
                if self.#ident != ::std::default::Default::default() {
                    #encode;
                }
            }
        },
    }
}

fn encode_optional_scalar_field(ident: &syn::Ident, ty: ScalarType, tag: u32) -> Tokens {
    let wire_type = ty.wire_type();
    let encode_key = quote!(_proto::encoding::encode_key(#tag, #wire_type, buf));
    let encode = ty.encode(&syn::Ident::new(format!("*self.{}", ident)));

    quote! {
        if let Some(ref value) = self.#ident {
            #encode_key;
            #encode;
        }
    }
}

fn encode_required_scalar_field(ident: &syn::Ident, ty: ScalarType, tag: u32) -> Tokens {
    let wire_type = ty.wire_type();
    let encode_key = quote!(_proto::encoding::encode_key(#tag, #wire_type, buf));
    let encode = ty.encode(&syn::Ident::new(format!("self.{}", ident)));

    quote! {
        #encode_key;
        #encode;
    }
}

fn encode_repeated_scalar_field(ident: &syn::Ident, ty: ScalarType, tag: u32) -> Tokens {
    let wire_type = ty.wire_type();
    let encode_key = quote!(_proto::encoding::encode_key(#tag, #wire_type, buf));
    let encode = ty.encode(&syn::Ident::new("value".to_string()));

    quote! {
        for &value in &self.#ident {
            #encode_key;
            #encode;
        }
    }
}

fn encode_packed_scalar_field(ident: &syn::Ident, ty: ScalarType, tag: u32) -> Tokens {
    quote!(();)
}

enum Field {
    /// A scalar field.
    Scalar {
        ident: syn::Ident,
        ty: ScalarType,
        tag: u32,
        label: Option<Label>,
        default: Option<syn::Lit>,
        packed: bool,
    },
    /// A message field.
    Message {
        ident: syn::Ident,
        tag: u32,
        label: Label,
    },
    /// A map field.
    Map {
        ident: syn::Ident,
        tag: u32,
        key_type: ScalarType,
        value_type: ScalarType,
    },
    /// A oneof field.
    Oneof {
        ident: syn::Ident,
        tags: Vec<u32>,
    },
}

impl Field {

    /// Converts an iterator of meta items into a field descriptor.
    ///
    /// If the meta items are invalid, an error will be returned.
    /// If the field should be ignored, `None` is returned.
    fn new(ident: syn::Ident, attrs: &[syn::Attribute]) -> Result<Option<Field>> {

        fn lit_to_scalar_type(lit: &syn::Lit) -> Result<ScalarType> {
            let s = if let syn::Lit::Str(ref s, _) = *lit {
                s
            } else {
                bail!("invalid type: {:?}", lit);
            };

            ScalarType::from_str(s).map(|kind| Ok(kind))
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

        fn set_option<T>(option: &mut Option<T>, value: T, message: &str) -> Result<()>
        where T: fmt::Debug {
            if let Some(ref existing) = *option {
                bail!("{}: {:?} and {:?}", message, existing, value);
            }
            *option = Some(value);
            Ok(())
        }

        // Common options.
        let mut tag = None;
        let mut label = None;

        // Scalar field options.
        let mut scalar_type = None;
        let mut packed = None;
        let mut default = None;

        // Message field optoins
        let mut message = false;

        // Map field options.
        let mut map = false;
        let mut key_type = None;
        let mut value_type = None;

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
                    if word.eq_ignore_ascii_case("ignore") { return Ok(None); }
                    else if word.eq_ignore_ascii_case("message") { message = true; }
                    else if word.eq_ignore_ascii_case("map") { map = true; }
                    else if word.eq_ignore_ascii_case("oneof") { oneof = true; }
                    else if let Some(ty) = ScalarType::from_str(word) {
                        set_option(&mut scalar_type, ty, "duplicate type attributes")?;
                    } else if let Some(l) = Label::from_str(word) {
                        set_option(&mut label, l, "duplicate label attributes")?;
                    } else {
                        bail!("unknown attribute: {}", word);
                    }
                },
                syn::NestedMetaItem::MetaItem(syn::MetaItem::NameValue(ref name, ref value)) => {
                    let name = name.as_ref();
                    if name.eq_ignore_ascii_case("tag") {
                        let t = lit_to_tag(&value).chain_err(|| "invalid tag attribute")?;
                        set_option(&mut tag, t, "duplicate tag attributes")?;
                    } else if name.eq_ignore_ascii_case("tags") {
                        let ts = lit_to_tags(&value).chain_err(|| "invalid tags attribute")?;
                        set_option(&mut tags, ts, "invalid tags attributes");
                    } else if name.eq_ignore_ascii_case("key") {
                        let kt = lit_to_scalar_type(&value).chain_err(|| "invalid map key type attribute")?;
                        set_option(&mut key_type, kt, "duplicate map key type attributes")?;
                    } else if name.eq_ignore_ascii_case("value") {
                        let vt = lit_to_scalar_type(&value).chain_err(|| "invalid map value type attribute")?;
                        set_option(&mut value_type, vt, "duplicate map value type attributes")?;
                    } else if name.eq_ignore_ascii_case("packed") {
                        let p = lit_to_bool(&value).chain_err(|| "illegal packed attribute")?;
                        set_option(&mut packed, p, "duplicate packed attributes")?;
                    } else if name.eq_ignore_ascii_case("default") {
                        set_option(&mut default, value, "duplicate default attributes")?;
                    }
                },
                syn::NestedMetaItem::Literal(ref lit) => bail!("invalid field attribute: {:?}", lit),
                syn::NestedMetaItem::MetaItem(syn::MetaItem::List(ref ident, _)) => bail!("invalid field attribute: {}", ident),
            }
        }

        // Check that either the field is a scalar type, a message, a map, or a oneof.
        match (scalar_type, message, map, oneof) {
            (Some(_), false, false, false) | (None, true, false, false) | (None, false, true, false) | (None, false, false, true) => (),
            (Some(ty), true, _, _) => bail!("duplicate type attributes: {} and message", ty),
            (Some(ty), _, true, _) => bail!("duplicate type attributes: {} and map", ty),
            (Some(ty), _, _, true) => bail!("duplicate type attributes: {} and oneof", ty),
            (_, true, true, _) => bail!("duplicate type attributes: message and map"),
            (_, true, _, true) => bail!("duplicate type attributes: message and oneof"),
            (_, _, true, true) => bail!("duplicate type attributes: map and oneof"),
            (None, false, false, false) => bail!("field must have a type attribute"),
        }

        let field = if let Some(ty) = scalar_type {
            if key_type.is_some() { bail!("invalid key type attribute for {} field", ty); }
            if value_type.is_some() { bail!("invalid value type attribute for {} field", ty); }
            if tags.is_some() { bail!("invalid tags attribute for {} field", ty); }

            let tag = match tag {
                Some(tag) => tag,
                None => bail!("{} field must have a tag attribute", ty),
            };

            if let Some(packed) = packed {
                match label {
                    Some(Label::Repeated) => (),
                    _ => bail!("packed attribute may only be applied to repeated fields"),
                }
                match ty  {
                    ScalarType::String | ScalarType::Bytes => {
                        bail!("packed attribute may only be applied to numeric fields");
                    },
                    _ => (),
                }
            }

            Field::Scalar {
                ident: ident,
                ty: ty,
                label: label,
                tag: tag,
                default: default.cloned(),
                packed: packed.unwrap_or(false),
            }
        } else if message {
            if key_type.is_some() { bail!("invalid key type attribute for message field"); }
            if value_type.is_some() { bail!("invalid value type attribute for message field"); }
            if tags.is_some() { bail!("invalid tags attribute for message field"); }
            if packed.is_some() { bail!("invalid packed attribute for message field"); }
            if default.is_some() { bail!("invalid default attribute for message field"); }

            let tag = match tag {
                Some(tag) => tag,
                None => bail!("message field must have a tag attribute"),
            };

            Field::Message {
                ident: ident,
                label: label.unwrap_or(Label::Optional),
                tag: tag,
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

            let key_type = match key_type {
                Some(key_type) => key_type,
                None => bail!("map field must have a key type attribute"),
            };

            let value_type = match value_type {
                Some(value_type) => value_type,
                None => bail!("map field must have a value type attribute"),
            };

            Field::Map {
                ident: ident,
                key_type: key_type,
                value_type: value_type,
                tag: tag,
            }
        } else {
            assert!(oneof);
            if let Some(label) = label { bail!("invalid {} attribute for oneof field", label); }
            if packed.is_some() { bail!("invalid packed attribute for oneof field"); }
            if default.is_some() { bail!("invalid default attribute for oneof field"); }
            if tag.is_some() { bail!("invalid tag attribute for oneof field"); }
            if key_type.is_some() { bail!("invalid key type attribute for oneof field"); }
            if value_type.is_some() { bail!("invalid value type attribute for oneof field"); }

            let tags = match tags {
                Some(tags) => tags,
                None => bail!("oneof field must have a tags attribute"),
            };

            Field::Oneof {
                ident: ident,
                tags: tags,
            }
        };

        Ok(Some(field))
    }

    fn ident(&self) -> &syn::Ident {
        match *self {
            Field::Scalar { ref ident, .. } => ident,
            Field::Message { ref ident, .. } => ident,
            Field::Map { ref ident, .. } => ident,
            Field::Oneof { ref ident, .. } => ident,
        }
    }

    fn tags(&self) -> Vec<u32> {
        match *self {
            Field::Scalar { tag, .. } => vec![tag],
            Field::Message { tag, .. } => vec![tag],
            Field::Map { tag, .. } => vec![tag],
            Field::Oneof { ref tags, .. } => tags.clone(),
        }
    }

    fn encode(&self) -> Tokens {
        match *self {
            Field::Scalar { ref ident, ty, tag, label, ref default , packed, .. } => {
                match label {
                    None => encode_scalar_field(ident, ty, tag, default.as_ref()),
                    Some(Label::Optional) => encode_optional_scalar_field(ident, ty, tag),
                    Some(Label::Required) => encode_required_scalar_field(ident, ty, tag),
                    Some(Label::Repeated) if packed => encode_packed_scalar_field(ident, ty, tag),
                    Some(Label::Repeated) => encode_repeated_scalar_field(ident, ty, tag),
                }
            },
            Field::Message { ref ident, tag, .. } => {
                quote! {
                    let len = self.#ident.encoded_len();
                    _proto::encoding::encode_key(#tag, _proto::encoding::WireType::Varint, buf);
                    _proto::encoding::encode_varint(len as u64, buf);
                    self.#ident.encode_length_delimited(buf)
                }
                quote!(();)
            }
            Field::Map { tag, .. } => {
                quote!(();)
            }
            Field::Oneof { .. } => {
                quote!(();)
            }
        }
    }

    fn merge(&self, tag: &syn::Ident, wire_type: &syn::Ident) -> Tokens {
        quote!(Ok(()))
    }

    fn encoded_len(&self) -> Tokens {
        quote!(0)
    }

    fn default(&self) -> Tokens {
        let ident = self.ident();

        quote!(#ident: ::std::default::Default::default(),)
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

    let fields = fields.into_iter()
                       .enumerate()
                       .flat_map(|(idx, field)| {
                           let field_ident = field.ident
                                                   .unwrap_or_else(|| syn::Ident::new(idx.to_string()));
                           match Field::new(field_ident.clone(), &field.attrs) {
                               Ok(Some(field)) => Some(Ok(field)),
                               Ok(None) => None,
                               Err(err) => Some(Err(err).chain_err(|| {
                                   format!("invalid message field {}.{}",
                                           ident, field_ident)
                               })),
                           }
                       })
                       .collect::<Result<Vec<Field>>>()?;

    let mut tags = fields.iter().flat_map(|field| field.tags()).collect::<Vec<_>>();
    let num_tags = tags.len();
    tags.sort();
    tags.dedup();
    if tags.len() != num_tags {
        bail!("message {} has fields with duplicate tags", ident);
    }

    let dummy_const = syn::Ident::new(format!("_IMPL_MESSAGE_FOR_{}", ident));

    let encoded_len = fields.iter()
                            .map(Field::encoded_len)
                            .fold(quote!(0), |mut sum, expr| {
                                sum.append("+");
                                sum.append(expr.as_str());
                                sum
                            });

    let encode = fields.iter().map(Field::encode).fold(Tokens::new(), concat_tokens);

    let merge = fields.iter().map(|field| {
        let merge = field.merge(&syn::Ident::new("tag"), &syn::Ident::new("wire_type"));
        let tags = field.tags().iter().map(|tag| quote!(#tag)).intersperse(quote!(|)).fold(Tokens::new(), concat_tokens);
        let field_ident = field.ident();
        //quote! { #tags => { #merge }.map_err(|error| {
            //::std::io::Error::new(
                //error.kind(),
                //format!(concat!("failed to decode field ", stringify!(#ident), ".", stringify!(#field_ident), ": {}"),
                        //error))
        //})?, }
        quote!( #tags => (), )
    }).fold(Tokens::new(), concat_tokens);

    let default = fields.iter()
                        .map(Field::default)
                        .fold(Tokens::new(), concat_tokens);

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

    expanded.parse::<TokenStream>().map_err(|err| Error::from(format!("{:?}", err)))
}

#[proc_macro_derive(Message, attributes(proto))]
pub fn message(input: TokenStream) -> TokenStream {
    try_message(input).unwrap()
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
