use std::ascii::AsciiExt;
use std::fmt;

use quote::Tokens;
use syn;

use error::*;
use field::Label;

/// A scalar protobuf field.
pub struct Field {
    pub ident: syn::Ident,
    pub ty: Ty,
    pub kind: Kind,
    pub tag: u32,
}

impl Field {

    /// Creates a new scalar field.
    pub fn new(ident: syn::Ident,
               ty: Ty,
               tag: u32,
               label: Option<Label>,
               default: Option<syn::Lit>,
               packed: bool) -> Result<Field> {

        let kind = match (label, packed, default.is_some()) {

            (None, false, _) => Kind::Plain(default),
            (Some(Label::Optional), false, false) => Kind::Optional,
            (Some(Label::Required), false, _) => Kind::Required(default),
            (Some(Label::Repeated), false, false) => Kind::Repeated,
            (Some(Label::Repeated), true, false) => {
                if ty.is_length_delimited() {
                    bail!("packed attribute may only be applied to numeric fields")
                }
                Kind::Packed
            },
            (_, true, _) => bail!("packed attribute may only be applied to repeated fields"),
            (Some(Label::Repeated), _, true) => bail!("repeated fields may not have a default value"),
            (Some(Label::Optional), _, true) => bail!("optional fields may not have a default value"),
        };

        Ok(Field {
            ident: ident,
            ty: ty,
            kind: kind,
            tag: tag,
        })
    }

    /// Returns a statement which encodes the scalar field.
    pub fn encode(&self) -> Tokens {
        let tag = self.tag;

        let field = syn::Ident::new(format!("self.{}", self.ident));
        let wire_type = self.ty.wire_type();
        let encode_key = quote!(_proto::encoding::encode_key(#tag, #wire_type, buf););
        let encode = self.ty.encode(&field);


        let value = syn::Ident::new(format!("value"));
        let encode_value = self.ty.encode(&value);


        match self.kind {
            Kind::Plain(Some(ref default)) => quote! {
                if #field != #default {
                    #encode_key
                    #encode
                }
            },
            Kind::Plain(None) => quote! {
                if #field != ::std::default::Default::default() {
                    #encode_key
                    #encode
                }
            },
            Kind::Optional => quote! {
                if let Some(value) = #field {
                    #encode_key
                    #encode_value
                }
            },
            Kind::Required(..) => quote! {
                #encode_key
                #encode
            },
            Kind::Repeated => {
                quote! {
                    for value in #field {
                        #encode_key
                        #encode_value
                    }
                }
            },
            Kind::Packed => {
                let len = if let Some(len) = self.ty.fixed_encoded_len() {
                    quote!(#field.len() as u64 * #len)
                } else {
                    let encoded_len = self.ty.encoded_len(&value);
                    quote!(#field.iter().map(|value| #encoded_len).sum() as u64)
                };

                quote! {
                    _proto::encoding::encode_key(#tag, _proto::encoding::WireType::LengthDelimited, buf);
                    _proto::encoding::encode_varint(#len, buf);
                    for value in #field {
                        #encode_value
                    }
                }
            },
        }
    }
}

/// A scalar protobuf field type.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Ty {
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

impl Ty {

    /// Returns the type as it appears in protobuf field declarations.
    pub fn as_str(&self) -> &'static str {
        match *self {
            Ty::Double => "double",
            Ty::Float => "double",
            Ty::Int32 => "int32",
            Ty::Int64 => "int64",
            Ty::Uint32 => "uint32",
            Ty::Uint64 => "uint64",
            Ty::Sint32 => "sint32",
            Ty::Sint64 => "sint64",
            Ty::Fixed32 => "fixed32",
            Ty::Fixed64 => "fixed64",
            Ty::Sfixed32 => "sfixed32",
            Ty::Sfixed64 => "sfixed64",
            Ty::Bool => "bool",
            Ty::String => "string",
            Ty::Bytes => "bytes",
            Ty::Enum => "enum",
        }
    }

    pub fn variants() -> &'static [Ty] {
        const VARIANTS: &'static [Ty] = &[
            Ty::Double,
            Ty::Float,
            Ty::Int32,
            Ty::Int64,
            Ty::Uint32,
            Ty::Uint64,
            Ty::Sint32,
            Ty::Sint64,
            Ty::Fixed32,
            Ty::Fixed64,
            Ty::Sfixed32,
            Ty::Sfixed64,
            Ty::Bool,
            Ty::String,
            Ty::Bytes,
            Ty::Enum,
        ];
        VARIANTS
    }

    /// Parses a string into a field type.
    /// If the string doesn't match a field type, `None` is returned.
    pub fn from_str(s: &str) -> Option<Ty> {
        for &kind in Ty::variants() {
            if s.eq_ignore_ascii_case(kind.as_str()) {
                return Some(kind);
            }
        }
        None
    }

    /// Returns a statement which encodes the scalar field with the provided name.
    fn encode(&self, ident: &syn::Ident) -> Tokens {
        match *self {
            Ty::Double => quote!(_proto::encoding::encode_double(#ident, buf);),
            Ty::Float => quote!(_proto::encoding::encode_float(#ident, buf);),
            Ty::Int32 => quote!(_proto::encoding::encode_int32(#ident, buf);),
            Ty::Int64 => quote!(_proto::encoding::encode_int64(#ident, buf);),
            Ty::Uint32 => quote!(_proto::encoding::encode_uint32(#ident, buf);),
            Ty::Uint64 => quote!(_proto::encoding::encode_uint64(#ident, buf);),
            Ty::Sint32 => quote!(_proto::encoding::encode_sint32(#ident, buf);),
            Ty::Sint64 => quote!(_proto::encoding::encode_sint64(#ident, buf);),
            Ty::Fixed32 => quote!(_proto::encoding::encode_fixed32(#ident, buf);),
            Ty::Fixed64 => quote!(_proto::encoding::encode_fixed64(#ident, buf);),
            Ty::Sfixed32 => quote!(_proto::encoding::encode_sfixed32(#ident, buf);),
            Ty::Sfixed64 => quote!(_proto::encoding::encode_sfixed64(#ident, buf);),
            Ty::Bool => quote!(_proto::encoding::encode_bool(#ident, buf);),
            Ty::String => quote!(_proto::encoding::encode_string(&#ident[..], buf);),
            Ty::Bytes => quote!(_proto::encoding::encode_bytes(&#ident[..], buf);),
            Ty::Enum => quote!(_proto::encoding::encode_int32(#ident as i32, buf);),
        }
    }

    /// Returns an expression which evaluates to the encoded length of the value.
    fn encoded_len(&self, ident: &syn::Ident) -> Tokens {
        match *self {
            Ty::Bool => quote!(1),
            Ty::Float | Ty::Fixed32 | Ty::Sfixed32 => quote!(4),
            Ty::Double | Ty::Fixed64 | Ty::Sfixed64 => quote!(8),
            Ty::Int32 => quote!(_proto::encoding::encoded_len_int32(#ident)),
            Ty::Int64 => quote!(_proto::encoding::encoded_len_int64(#ident)),
            Ty::Uint32 => quote!(_proto::encoding::encoded_len_uint32(#ident)),
            Ty::Uint64 => quote!(_proto::encoding::encoded_len_uint64(#ident)),
            Ty::Sint32 => quote!(_proto::encoding::encoded_len_sint32(#ident)),
            Ty::Sint64 => quote!(_proto::encoding::encoded_len_sint64(#ident)),
            Ty::Enum => quote!(_proto::encoding::encoded_len_varint(#ident as u64)),
            Ty::String | Ty::Bytes => quote! {
                {
                    let len = #ident.len();
                    len + _proto::encoding::encoded_len_varint(len as u64)
                }
            },
        }
    }

    /// Returns the encoded length of the type, if it's not value-dependent.
    fn fixed_encoded_len(&self) -> Option<usize> {
        match *self {
            Ty::Bool => Some(1),
            Ty::Float | Ty::Fixed32 | Ty::Sfixed32 => Some(4),
            Ty::Double | Ty::Fixed64 | Ty::Sfixed64 => Some(8),
            _ => None,
        }
    }

    /// Returns an expression which evaluates to the wire type of the scalar type.
    fn wire_type(&self) -> Tokens {
        match *self {
            Ty::Float
                | Ty::Fixed32
                | Ty::Sfixed32 => quote!(_proto::encoding::WireType::ThirtyTwoBit),
            Ty::Double
                | Ty::Fixed64
                | Ty::Sfixed64 => quote!(_proto::encoding::WireType::SixtyFourBit),
            Ty::Int32
                | Ty::Int64
                | Ty::Uint32
                | Ty::Uint64
                | Ty::Sint32
                | Ty::Sint64
                | Ty::Bool
                | Ty::Enum => quote!(_proto::encoding::WireType::Varint),
            Ty::String
                | Ty::Bytes => quote!(_proto::encoding::WireType::LengthDelimited),
        }
    }

    /// Returns true if the scalar type is length delimited (i.e., `string` or `bytes`).
    fn is_length_delimited(&self) -> bool {
        *self == Ty::String || *self == Ty::Bytes
    }
}

impl fmt::Debug for Ty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Scalar protobuf field types.
pub enum Kind {
    /// A plain proto3 scalar field with an optional default value.
    Plain(Option<syn::Lit>),
    /// An optional scalar field.
    Optional,
    /// A required proto2 scalar field with an optional default value.
    Required(Option<syn::Lit>),
    /// A repeated scalar field.
    Repeated,
    /// A packed repeated scalar field.
    Packed,
}
