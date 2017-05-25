use std::fmt;

use error_chain::ChainedError;
use quote::{self, Tokens};
use syn::{
    FloatTy,
    Ident,
    IntTy,
    Lit,
    MetaItem,
    NestedMetaItem,
    StrStyle,
};

use error::*;
use field::{
    Label,
    bool_attr,
    set_option,
    tag_attr,
};

/// A scalar protobuf field.
pub struct Field {
    pub ident: Ident,
    pub ty: Ty,
    pub kind: Kind,
    pub tag: u32,
}

impl Field {

    pub fn new(ident: &Ident, attrs: &[MetaItem]) -> Result<Option<Field>> {
        let mut ty = None;
        let mut label = None;
        let mut packed = None;
        let mut default = None;
        let mut tag = None;

        let mut unknown_attrs = Vec::new();

        for attr in attrs {
            if let Some(t) = Ty::from_attr(attr)? {
                set_option(&mut ty, t, "duplicate type attributes")?;
            } else if let Some(p) = bool_attr("packed", attr)? {
                set_option(&mut packed, p, "duplicate packed attributes")?;
            } else if let Some(t) = tag_attr(attr)? {
                set_option(&mut tag, t, "duplicate tag attributes")?;
            } else if let Some(l) = Label::from_attr(attr) {
                set_option(&mut label, l, "duplicate label attributes")?;
            } else if let Some(d) = DefaultValue::from_attr(attr)? {
                set_option(&mut default, d, "duplicate default attributes")?;
            } else {
                unknown_attrs.push(attr);
            }
        }

        let ty = match ty {
            Some(ty) => ty,
            None => return Ok(None),
        };

        match unknown_attrs.len() {
            0 => (),
            1 => bail!("unknown attribute for {} field: {:?}", ty, unknown_attrs[0]),
            _ => bail!("unknown attributes for {} field: {:?}", ty, unknown_attrs),
        }

        let tag = match tag {
            Some(tag) => tag,
            None => bail!("{} field is missing a tag attribute", ty),
        };

        let has_default = default.is_some();
        let default = default.unwrap_or_else(|| DefaultValue::new(&ty));
        let kind = match (label, packed, has_default) {
            (None, Some(true), _) |
            (Some(Label::Optional), Some(true), _) |
            (Some(Label::Required), Some(true), _) => {
                bail!("packed attribute may only be applied to repeated fields");
            },
            (Some(Label::Repeated), Some(true), _) if !ty.is_numeric() => {
                bail!("packed attribute may only be applied to numeric fields");
            },
            (Some(Label::Repeated), _, true) => {
                bail!("repeated fields may not have a default value");
            },

            (None, _, _) => Kind::Plain(default),
            (Some(Label::Optional), _, _) => Kind::Optional(default),
            (Some(Label::Required), _, _) => Kind::Required(default),
            (Some(Label::Repeated), packed, false) if packed.unwrap_or(ty.is_numeric()) => Kind::Packed,
            (Some(Label::Repeated), _, false) => Kind::Repeated,
        };

        Ok(Some(Field {
            ident: ident.clone(),
            ty: ty,
            kind: kind,
            tag: tag,
        }))
    }

    /// Returns a statement which encodes the scalar field.
    pub fn encode(&self) -> Tokens {
        let encode_fn = self.ty.encode_fn(&self.kind);
        let tag = self.tag;
        let field = Ident::new(format!("self.{}", self.ident));

        match self.kind {
            Kind::Plain(ref default) => quote! {
                if #field != #default {
                    #encode_fn(#tag, &#field, buf);
                }
            },
            Kind::Optional(ref default) => quote! {
                if let Some(ref value) = #field {
                    if value != #default {
                        #encode_fn(#tag, &value, buf);
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
    pub fn merge(&self, wire_type: &Ident) -> Tokens {
        let merge_fn = self.ty.merge_fn(&self.kind);
        let field = Ident::new(format!("self.{}", self.ident));

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
        let encoded_len_fn = self.ty.encoded_len_fn(&self.kind);
        let tag = self.tag;
        let field = Ident::new(format!("self.{}", self.ident));

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
                        #encoded_len_fn(#tag, &value)
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
            Kind::Plain(ref value) | Kind::Required(ref value) => value.owned(),
            Kind::Optional(_) => quote!(::std::option::Option::None),
            Kind::Repeated | Kind::Packed => quote!(::std::vec::Vec::new()),
        }
    }

    /// Returns methods to embed in the message.
    pub fn methods(&self) -> Option<Tokens> {
        let ident = &self.ident;
        let set = Ident::new(format!("set_{}", ident));
        let push = Ident::new(format!("push_{}", ident));

        if let Ty::Enumeration(ref ty) = self.ty {
            Some(match self.kind {
                Kind::Plain(..) | Kind::Required(..) => {
                    quote! {
                        fn #ident(&self) -> ::std::option::Option<#ty> {
                            #ty::from_i32(self.#ident)
                        }

                        fn #set(&mut self, value: #ty) {
                            self.#ident = value as i32;
                        }
                    }
                },
                Kind::Optional(..) => {
                    quote! {
                        fn #ident(&self) -> ::std::option::Option<#ty> {
                            self.#ident.and_then(#ty::from_i32)
                        }

                        fn #set(&mut self, value: #ty) {
                            self.#ident = ::std::option::Some(value as i32);
                        }
                    }
                },
                Kind::Repeated | Kind::Packed => {
                    quote! {
                        fn #ident(&self) -> ::std::iter::FilterMap<::std::iter::Cloned<::std::slice::Iter<i32>>,
                                                                   fn(i32) -> Option<#ty>> {
                            self.#ident.iter().cloned().filter_map(#ty::from_i32)
                        }
                        fn #push(&mut self, value: #ty) {
                            self.#ident.push(value as i32);
                        }
                    }
                },
            })
        } else {
            None
        }
    }
}

/// A scalar protobuf field type.
#[derive(Clone, PartialEq, Eq)]
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
    Enumeration(Ident),
}

impl Ty {

    pub fn from_attr(attr: &MetaItem) -> Result<Option<Ty>> {
        let ty = match *attr {
            MetaItem::Word(ref name) if name == "float" => Ty::Float,
            MetaItem::Word(ref name) if name == "double" => Ty::Double,
            MetaItem::Word(ref name) if name == "int32" => Ty::Int32,
            MetaItem::Word(ref name) if name == "int64" => Ty::Int64,
            MetaItem::Word(ref name) if name == "uint32" => Ty::Uint32,
            MetaItem::Word(ref name) if name == "uint64" => Ty::Uint64,
            MetaItem::Word(ref name) if name == "sint32" => Ty::Sint32,
            MetaItem::Word(ref name) if name == "sint64" => Ty::Sint64,
            MetaItem::Word(ref name) if name == "fixed32" => Ty::Fixed32,
            MetaItem::Word(ref name) if name == "fixed64" => Ty::Fixed64,
            MetaItem::Word(ref name) if name == "sfixed32" => Ty::Sfixed32,
            MetaItem::Word(ref name) if name == "sfixed64" => Ty::Sfixed64,
            MetaItem::Word(ref name) if name == "bool" => Ty::Bool,
            MetaItem::Word(ref name) if name == "string" => Ty::String,
            MetaItem::Word(ref name) if name == "bytes" => Ty::Bytes,
            MetaItem::NameValue(ref name, Lit::Str(ref ident, _)) if name == "enumeration" => {
                Ty::Enumeration(Ident::new(ident.as_ref()))
            },
            MetaItem::List(ref name, ref items) if name == "enumeration" => {
                // TODO(rustlang/rust#23121): slice pattern matching would make this much nicer.
                if items.len() == 1 {
                    if let NestedMetaItem::MetaItem(MetaItem::Word(ref ident)) = items[0] {
                        Ty::Enumeration(ident.clone())
                    } else {
                        bail!("invalid enumeration attribute: item must be an identifier");
                    }
                } else {
                    bail!("invalid enumeration attribute: only a single identifier is supported");
                }
            },
            _ => return Ok(None),
        };
        Ok(Some(ty))
    }

    pub fn from_str(s: &str) -> Result<Ty> {
        let enumeration_len = "enumeration".len();
        let error = Err(From::from(format!("invalid type: {}", s)));
        let ty = match s.trim() {
            "float" => Ty::Float,
            "double" => Ty::Double,
            "int32" => Ty::Int32,
            "int64" => Ty::Int64,
            "uint32" => Ty::Uint32,
            "uint64" => Ty::Uint64,
            "sint32" => Ty::Sint32,
            "sint64" => Ty::Sint64,
            "fixed32" => Ty::Fixed32,
            "fixed64" => Ty::Fixed64,
            "sfixed32" => Ty::Sfixed32,
            "sfixed64" => Ty::Sfixed64,
            "bool" => Ty::Bool,
            "string" => Ty::String,
            "bytes" => Ty::Bytes,
            s if s.len() > enumeration_len && &s[..enumeration_len] == "enumeration" => {
                let s = &s[enumeration_len..].trim();
                match s.chars().next() {
                    Some('<') | Some('(') => (),
                    _ => return error,
                }
                match s.chars().next_back() {
                    Some('>') | Some(')') => (),
                    _ => return error,
                }
                Ty::Enumeration(Ident::new(s[1..s.len() - 1].trim()))
            },
            _ => return error,
        };
        Ok(ty)
    }

    /// Returns the type as it appears in protobuf field declarations.
    pub fn as_str(&self) -> &'static str {
        match *self {
            Ty::Double => "double",
            Ty::Float => "float",
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
            Ty::Enumeration(..) => "enum",
        }
    }

    pub fn encode_as(&self) -> &'static str {
        match *self {
            Ty::Enumeration(..) => "int32",
            _ => self.as_str(),
        }
    }

    pub fn encode_fn(&self, kind: &Kind) -> Ident {
        let kind = match *kind {
            Kind::Plain(..) | Kind::Optional(..) | Kind::Required(..) => "",
            Kind::Repeated => "repeated_",
            Kind::Packed => "packed_",
        };


        let ty = self.encode_as();
        Ident::new(format!("_proto::encoding::encode_{}{}", kind, ty))
    }

    pub fn merge_fn(&self, kind: &Kind) -> Ident {
        let kind = match *kind {
            Kind::Plain(..) | Kind::Optional(..) | Kind::Required(..) => "",
            Kind::Repeated | Kind::Packed => "repeated_",
        };
        let ty = self.encode_as();
        Ident::new(format!("_proto::encoding::merge_{}{}", kind, ty))
    }

    pub fn encoded_len_fn(&self, kind: &Kind) -> Ident {
        let kind = match *kind {
            Kind::Plain(..) | Kind::Optional(..) | Kind::Required(..) => "",
            Kind::Repeated => "repeated_",
            Kind::Packed => "packed_",
        };
        let ty = self.encode_as();
        Ident::new(format!("_proto::encoding::encoded_len_{}{}", kind, ty))
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

#[derive(Debug)]
pub enum DefaultValue {
    Lit(Lit),
    Ident(Ident),
}

impl DefaultValue {

    pub fn from_attr(attr: &MetaItem) -> Result<Option<DefaultValue>> {
        if attr.name() != "default" {
            return Ok(None);
        }
        match *attr {
            MetaItem::List(_, ref items) => {
                // TODO(rustlang/rust#23121): slice pattern matching would make this much nicer.
                if items.len() == 1 {
                    if let Some(&NestedMetaItem::Literal(ref lit)) = items.first() {
                        return Ok(Some(DefaultValue::Lit(lit.clone())));
                    }
                }
                bail!("invalid default value attribute: {:?}", attr);
            },
            MetaItem::NameValue(_, ref lit) => {
                match *lit {
                    Lit::Str(ref s, _) => return Ok(Some(DefaultValue::Ident(Ident::new(s.as_str())))),
                    _ => bail!("invalid default value attribute: {:?}", attr),
                }
            },
            _ => bail!("invalid tag attribute: {:?}", attr),
        }
    }

    pub fn new(ty: &Ty) -> DefaultValue {
        let lit = match *ty {
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
            Ty::Enumeration(ref ty) => return DefaultValue::Ident(Ident::new(format!("{}::default() as i32", ty))),
        };
        DefaultValue::Lit(lit)
    }

    fn from_lit(ty: &Ty, lit: Lit) -> DefaultValue {
        match lit {
            // If the default value is a string literal, and the type isn't a
            // string, assume the default is an expression which, when evaluated,
            // returns a value of the correct type.
            Lit::Str(ref value, ..) if ty != &Ty::String => DefaultValue::Ident(Ident::new(value.to_string())),
            // Otherwise, we assume the user has provided a literal of the correct type.
            // TODO: parse protobuf's hex encoding for bytes fields.
            _ => DefaultValue::Lit(lit),
        }
    }

    pub fn owned(&self) -> Tokens {
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
