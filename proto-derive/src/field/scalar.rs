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
            (None, false, _) => Kind::Plain(default.unwrap_or_else(|| ty.default())),
            (Some(Label::Optional), false, _) => Kind::Optional(default.unwrap_or_else(|| ty.default())),
            (Some(Label::Required), false, _) => Kind::Required(default.unwrap_or_else(|| ty.default())),
            (Some(Label::Repeated), false, false) => Kind::Repeated,
            (Some(Label::Repeated), true, false) => {
                if ty.is_length_delimited() {
                    bail!("packed attribute may only be applied to numeric fields")
                }
                Kind::Packed
            },
            (_, true, _) => bail!("packed attribute may only be applied to repeated fields"),
            (Some(Label::Repeated), _, true) => bail!("repeated fields may not have a default value"),
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
        let encode_fn = {
            let kind = match self.kind {
                Kind::Plain(..) | Kind::Optional(..) | Kind::Required(..) => "",
                Kind::Repeated => "repeated_",
                Kind::Packed => "packed_",
            };
            let ty = self.ty.as_str();
            syn::Ident::new(format!("_proto::encoding::encode_{}{}", kind, ty))
        };

        let tag = self.tag;
        let field = syn::Ident::new(format!("self.{}", self.ident));

        match self.kind {
            Kind::Plain(ref default) => quote! {
                if #field != #default {
                    #encode_fn(#tag, &#field, buf);
                }
            },
            // TODO: figure out if this is right.  Will the c++ proto2 skip encoding default values
            // even if they are set?
            Kind::Optional(ref default) => quote! {
                if let Some(ref value) = #field {
                    if value != #default {
                        #encode_fn(#tag, value, buf);
                    }
                }
            },
            Kind::Required(..) | Kind::Repeated | Kind::Packed => quote!{
                #encode_fn(#tag, &#field, buf);
            },
        }
    }

    /// Returns an expression which evaluates to the result of merging a decoded scalar value into
    /// the field.
    pub fn merge(&self, wire_type: &syn::Ident) -> Tokens {
        let merge_fn = {
            let kind = match self.kind {
                Kind::Plain(..) | Kind::Optional(..) | Kind::Required(..) => "",
                Kind::Repeated | Kind::Packed => "repeated_",
            };
            let ty = self.ty.as_str();
            syn::Ident::new(format!("_proto::encoding::merge_{}{}", kind, ty))
        };
        let field = syn::Ident::new(format!("self.{}", self.ident));

        match self.kind {
            Kind::Plain(..) | Kind::Required(..) | Kind::Repeated | Kind::Packed => quote! {
                #merge_fn(#wire_type, &mut #field, buf)
            },
            Kind::Optional(..) => quote! {
                {
                    let mut value = #field.take().unwrap_or_default();
                    #merge_fn(#wire_type, &mut value, buf).map(|_| #field = ::std::option::Option::Some(value))
                }
            },
        }
    }

    /// Returns an expression which evaluates to the encoded length of the field.
    pub fn encoded_len(&self) -> Tokens {
        let encoded_len_fn = {
            let kind = match self.kind {
                Kind::Plain(..) | Kind::Optional(..) | Kind::Required(..) => "",
                Kind::Repeated => "repeated_",
                Kind::Packed => "packed_",
            };
            let ty = self.ty.as_str();
            syn::Ident::new(format!("_proto::encoding::encoded_len_{}{}", kind, ty))
        };

        let tag = self.tag;
        let field = syn::Ident::new(format!("self.{}", self.ident));

        match self.kind {
            Kind::Plain(ref default) => quote! {
                if #field != #default {
                    #encoded_len_fn(#tag, &#field)
                } else {
                    0
                }
            },
            Kind::Optional(ref default) => quote! {
                #field.as_ref().map_or(0, |value| {
                    if value != #default {
                        #encoded_len_fn(#tag, value)
                    } else {
                        0
                    }
                })
            },
            Kind::Required(..) | Kind::Repeated | Kind::Packed => quote!{
                #encoded_len_fn(#tag, &#field)
            },
        }
    }

    /// Returns an expression which evaluates to the default value of the field.
    pub fn default(&self) -> Tokens {
        match self.kind {
            Kind::Plain(ref value) | Kind::Required(ref value) => {
                match *value {
                    syn::Lit::Str(ref value, _) => quote!(#value.to_string()),
                    _ => quote!(#value),
                }
            },
            Kind::Optional(ref value) => {
                match *value {
                    syn::Lit::Str(ref value, _) => quote!(::std::option::Option::Some(#value.to_string())),
                    _ => quote!(::std::option::Option::Some(#value.to_string())),
                }
            },
            Kind::Repeated | Kind::Packed => quote!(::std::vec::Vec::new()),
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

    pub fn default(&self) -> syn::Lit {
        match *self {
            Ty::Float => syn::Lit::Float("0.0".to_string(), syn::FloatTy::F32),
            Ty::Double => syn::Lit::Float("0.0".to_string(), syn::FloatTy::F64),
            Ty::Int32 => syn::Lit::Int(0, syn::IntTy::I32),
            Ty::Int64 => syn::Lit::Int(0, syn::IntTy::I64),
            Ty::Uint32 => syn::Lit::Int(0, syn::IntTy::U32),
            Ty::Uint64 => syn::Lit::Int(0, syn::IntTy::U64),
            Ty::Sint32 => syn::Lit::Int(0, syn::IntTy::I32),
            Ty::Sint64 => syn::Lit::Int(0, syn::IntTy::I64),
            Ty::Fixed32 => syn::Lit::Int(0, syn::IntTy::U32),
            Ty::Fixed64 => syn::Lit::Int(0, syn::IntTy::U64),
            Ty::Sfixed32 => syn::Lit::Int(0, syn::IntTy::I32),
            Ty::Sfixed64 => syn::Lit::Int(0, syn::IntTy::I64),
            Ty::Bool => syn::Lit::Bool(false),
            Ty::String => syn::Lit::Str(String::new(), syn::StrStyle::Cooked),
            Ty::Bytes => syn::Lit::ByteStr(Vec::new(), syn::StrStyle::Cooked),
            Ty::Enum => unimplemented!(),
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
    Plain(syn::Lit),
    /// An optional scalar field.
    Optional(syn::Lit),
    /// A required proto2 scalar field with an optional default value.
    Required(syn::Lit),
    /// A repeated scalar field.
    Repeated,
    /// A packed repeated scalar field.
    Packed,
}
