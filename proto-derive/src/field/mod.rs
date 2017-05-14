mod scalar;

use std::ascii::AsciiExt;
use std::fmt;
use std::slice;

use quote::Tokens;
use syn;

use error::*;

pub enum Field {
    /// A scalar field.
    Scalar(scalar::Field),
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
        key_type: scalar::Ty,
        value_type: scalar::Ty,
    },
    /// A oneof field.
    Oneof {
        ident: syn::Ident,
        tags: Vec<u32>,
    },
}

impl Field {

    /// Creates a new `Field` from an iterator of field attributes.
    ///
    /// If the meta items are invalid, an error will be returned.
    /// If the field should be ignored, `None` is returned.
    pub fn new(ident: syn::Ident, attrs: &[syn::Attribute]) -> Result<Option<Field>> {

        fn lit_to_scalar_type(lit: &syn::Lit) -> Result<scalar::Ty> {
            let s = if let syn::Lit::Str(ref s, _) = *lit {
                s
            } else {
                bail!("invalid type: {:?}", lit);
            };

            scalar::Ty::from_str(s).map(|kind| Ok(kind))
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
                    else if let Some(ty) = scalar::Ty::from_str(word) {
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
                        set_option(&mut tags, ts, "invalid tags attributes")?;
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

            Field::Scalar(scalar::Field::new(ident, ty, tag, label,
                                             default.cloned(), packed.unwrap_or(false))?)
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

    pub fn ident(&self) -> &syn::Ident {
        match *self {
            Field::Scalar(ref scalar) => &scalar.ident,
            Field::Message { ref ident, .. } => ident,
            Field::Map { ref ident, .. } => ident,
            Field::Oneof { ref ident, .. } => ident,
        }
    }

    pub fn tags(&self) -> Vec<u32> {
        match *self {
            Field::Scalar(ref scalar) => vec![scalar.tag],
            Field::Message { tag, .. } => vec![tag],
            Field::Map { tag, .. } => vec![tag],
            Field::Oneof { ref tags, .. } => tags.clone(),
        }
    }

    pub fn encode(&self) -> Tokens {
        match *self {
            Field::Scalar(ref scalar) => scalar.encode(),
            Field::Message { ref ident, tag, .. } => {
                quote! {
                    let len = self.#ident.encoded_len();
                    _proto::encoding::encode_key(#tag, _proto::encoding::WireType::Varint, buf);
                    _proto::encoding::encode_varint(len as u64, buf);
                    self.#ident.encode(buf)
                }
            }
            Field::Map { tag, .. } => {
                quote!(();)
            }
            Field::Oneof { .. } => {
                quote!(();)
            }
        }
    }

    pub fn merge(&self, tag: &syn::Ident, wire_type: &syn::Ident) -> Tokens {
        quote!(Ok(()))
    }

    pub fn encoded_len(&self) -> Tokens {
        quote!(0)
    }

    pub fn default(&self) -> Tokens {
        let ident = self.ident();

        quote!(#ident: ::std::default::Default::default(),)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Label {
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

/// Parses a literal value into a bool.
fn lit_to_bool(lit: &syn::Lit) -> Result<bool> {
    match *lit {
        syn::Lit::Bool(b) => Ok(b),
        syn::Lit::Str(ref s, _) => s.parse::<bool>().map_err(|e| Error::from(e.to_string())),
        _ => bail!("invalid literal value: {:?}", lit),
    }
}
