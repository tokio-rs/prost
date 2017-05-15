use std::ascii::AsciiExt;
use std::fmt;

use quote::{self, Tokens};
use syn::{
    Ident,
    Lit,
    FloatTy,
    IntTy,
    StrStyle,
};

use error::*;
use field::Label;

/// A scalar protobuf field.
pub struct Field {
    pub ident: Ident,
    pub ty: Ty,
    pub kind: Kind,
    pub tag: u32,
}

impl Field {

    /// Creates a new scalar field.
    pub fn new(ident: Ident,
               ty: Ty,
               tag: u32,
               label: Option<Label>,
               default: Option<Lit>,
               packed: bool) -> Result<Field> {

        let has_default = default.is_some();
        let default = default.map_or_else(|| DefaultValue::new(ty),
                                          |lit| DefaultValue::from_lit(ty, lit));

        let kind = match (label, packed, has_default) {
            (None, false, _) => Kind::Plain(default),
            (Some(Label::Optional), false, _) => Kind::Optional(default),
            (Some(Label::Required), false, _) => Kind::Required(default),
            (Some(Label::Repeated), false, false) => Kind::Repeated,
            (Some(Label::Repeated), true, false) => {
                if !ty.is_numeric() {
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
            let ty = self.ty.encode_as();
            Ident::new(format!("_proto::encoding::encode_{}{}", kind, ty))
        };

        let tag = self.tag;
        let field = Ident::new(format!("self.{}", self.ident));
        let cast = if self.ty == Ty::Enumeration { quote!(as i32) } else { quote!() };

        match self.kind {
            Kind::Plain(ref default) => quote! {
                if #field != #default {
                    #encode_fn(#tag, &(#field #cast), buf);
                }
            },
            // TODO: figure out if this is right.  Will the c++ proto2 skip encoding default values
            // even if they are set?
            Kind::Optional(ref default) => quote! {
                if let Some(ref value) = #field {
                    if value != #default {
                        #encode_fn(#tag, &(value #cast), buf);
                    }
                }
            },
            Kind::Required(..) | Kind::Repeated | Kind::Packed => quote!{
                #encode_fn(#tag, &(#field #cast), buf);
            },
        }
    }

    /// Returns an expression which evaluates to the result of merging a decoded scalar value into
    /// the field.
    pub fn merge(&self, wire_type: &Ident) -> Tokens {
        let merge_fn = {
            let kind = match self.kind {
                Kind::Plain(..) | Kind::Optional(..) | Kind::Required(..) => "",
                Kind::Repeated | Kind::Packed => "repeated_",
            };
            let ty = self.ty.encode_as();
            Ident::new(format!("_proto::encoding::merge_{}{}", kind, ty))
        };
        let field = Ident::new(format!("self.{}", self.ident));
        let cast = if self.ty == Ty::Enumeration { quote!(as i32) } else { quote!() };

        match self.kind {
            Kind::Plain(..) | Kind::Required(..) | Kind::Repeated | Kind::Packed => quote! {
                #merge_fn(#wire_type, &mut (#field #cast), buf)
            },
            Kind::Optional(..) => quote! {
                {
                    let mut value = #field.take().unwrap_or_default();
                    #merge_fn(#wire_type, &mut (value #cast), buf).map(|_| #field = ::std::option::Option::Some(value))
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
            let ty = self.ty.encode_as();
            Ident::new(format!("_proto::encoding::encoded_len_{}{}", kind, ty))
        };

        let tag = self.tag;
        let field = Ident::new(format!("self.{}", self.ident));
        let cast = if self.ty == Ty::Enumeration { quote!(as i32) } else { quote!() };

        match self.kind {
            Kind::Plain(ref default) => quote! {
                if #field != #default {
                    #encoded_len_fn(#tag, &(#field #cast))
                } else {
                    0
                }
            },
            Kind::Optional(ref default) => quote! {
                #field.as_ref().map_or(0, |value| {
                    if value != #default {
                        #encoded_len_fn(#tag, &(value #cast))
                    } else {
                        0
                    }
                })
            },
            Kind::Required(..) | Kind::Repeated | Kind::Packed => quote!{
                #encoded_len_fn(#tag, &(#field #cast))
            },
        }
    }

    /// Returns an expression which evaluates to the default value of the field.
    pub fn default(&self) -> Tokens {
        match self.kind {
            Kind::Plain(ref value) | Kind::Required(ref value) => value.owned(),
            Kind::Optional(ref value) => quote!(::std::option::Option::None),
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
    Enumeration,
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
            Ty::Enumeration => "enum",
        }
    }

    pub fn attr(&self) -> &'static str {
        match *self {
            Ty::Enumeration => "enumeration",
            _ => self.as_str(),
        }
    }

    pub fn encode_as(&self) -> &'static str {
        match *self {
            Ty::Enumeration => "int32",
            _ => self.as_str(),
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
            Ty::Enumeration,
        ];
        VARIANTS
    }

    /// Parses an attribute into a field type.
    /// If then attribute doesn't match a field type, `None` is returned.
    pub fn from_attr(attr: &str) -> Option<Ty> {
        for &ty in Ty::variants() {
            if attr.eq_ignore_ascii_case(ty.attr()) {
                return Some(ty);
            }
        }
        None
    }

    /// Returns true if the scalar type is length delimited (i.e., `string` or `bytes`).
    fn is_numeric(&self) -> bool {
        *self != Ty::String && *self != Ty::Bytes
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
    /// A plain proto3 scalar field.
    Plain(DefaultValue),
    /// An optional scalar field.
    Optional(DefaultValue),
    /// A required proto2 scalar field.
    Required(DefaultValue),
    /// A repeated scalar field.
    Repeated,
    /// A packed repeated scalar field.
    Packed,
}

pub enum DefaultValue {
    Lit(Lit),
    Ident(Ident),
}

impl DefaultValue {
    fn new(ty: Ty) -> DefaultValue {
        let lit = match ty {
            Ty::Float => Lit::Float("0.0".to_string(), FloatTy::F32),
            Ty::Double => Lit::Float("0.0".to_string(), FloatTy::F64),
            Ty::Int32 => Lit::Int(0, IntTy::I32),
            Ty::Int64 => Lit::Int(0, IntTy::I64),
            Ty::Uint32 => Lit::Int(0, IntTy::U32),
            Ty::Uint64 => Lit::Int(0, IntTy::U64),
            Ty::Sint32 => Lit::Int(0, IntTy::I32),
            Ty::Sint64 => Lit::Int(0, IntTy::I64),
            Ty::Fixed32 => Lit::Int(0, IntTy::U32),
            Ty::Fixed64 => Lit::Int(0, IntTy::U64),
            Ty::Sfixed32 => Lit::Int(0, IntTy::I32),
            Ty::Sfixed64 => Lit::Int(0, IntTy::I64),
            Ty::Bool => Lit::Bool(false),
            Ty::String => Lit::Str(String::new(), StrStyle::Cooked),
            Ty::Bytes => Lit::ByteStr(Vec::new(), StrStyle::Cooked),
            Ty::Enumeration => return DefaultValue::Ident(Ident::new("::std::default::Default::default()")),
        };
        DefaultValue::Lit(lit)
    }

    fn from_lit(ty: Ty, lit: Lit) -> DefaultValue {
        match lit {
            // If the default value is a string literal, and the type isn't a
            // string, assume the default is an expression which, when evaluated,
            // returns a value of the correct type.
            Lit::Str(ref value, ..) if ty != Ty::String => DefaultValue::Ident(Ident::new(value.to_string())),
            // Otherwise, we assume the user has provided a literal of the correct type.
            // TODO: parse protobuf's hex encoding for bytes fields.
            _ => DefaultValue::Lit(lit),
        }
    }

    fn owned(&self) -> Tokens {
        match *self {
            DefaultValue::Lit(Lit::Str(ref value, ..)) if value.is_empty() => quote!(::std::string::String::new()),
            DefaultValue::Lit(ref lit@Lit::Str(..)) => quote!(#lit.to_owned()),
            DefaultValue::Lit(Lit::ByteStr(ref value, ..)) if value.is_empty() => quote!(::std::vec::Vec::new()),
            DefaultValue::Lit(ref lit@Lit::ByteStr(..)) => quote!(#lit.to_owned()),
            _ => quote!(#self),
        }
    }
}

impl quote::ToTokens for DefaultValue {
    fn to_tokens(&self, tokens: &mut Tokens) {
        match *self {
            DefaultValue::Lit(ref lit) => lit.to_tokens(tokens),
            DefaultValue::Ident(ref ident) => ident.to_tokens(tokens),
        }
    }
}
