mod group;
mod map;
mod message;
mod oneof;
mod scalar;

use std::fmt;
use std::slice;

use anyhow::{bail, Error};
use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::Path;
use syn::{Attribute, Expr, ExprLit, Lit, LitBool, LitInt, Meta, MetaNameValue, Token};

/// A field of a message, or a variant of a oneof.
#[derive(Clone)]
pub struct Field {
    kind: Kind,
    /// Whether the field's value is hidden from the generated `Debug` output.
    debug_redact: bool,
}

#[derive(Clone)]
enum Kind {
    /// A scalar field.
    Scalar(scalar::Field),
    /// A message field.
    Message(message::Field),
    /// A map field.
    Map(map::Field),
    /// A oneof field.
    Oneof(oneof::Field),
    /// A group field.
    Group(group::Field),
}

impl Field {
    /// Creates a new `Field` from an iterator of field attributes.
    ///
    /// If the meta items are invalid, an error will be returned.
    /// If the field should be ignored, `None` is returned.
    pub fn new(attrs: Vec<Attribute>, inferred_tag: Option<u32>) -> Result<Option<Field>, Error> {
        let attrs = prost_attrs(attrs)?;
        let (attrs, debug_redact) = take_debug_redact(attrs)?;

        // TODO: check for ignore attribute.

        let kind = if let Some(field) = scalar::Field::new(&attrs, inferred_tag)? {
            Kind::Scalar(field)
        } else if let Some(field) = message::Field::new(&attrs, inferred_tag)? {
            Kind::Message(field)
        } else if let Some(field) = map::Field::new(&attrs, inferred_tag)? {
            Kind::Map(field)
        } else if let Some(field) = oneof::Field::new(&attrs)? {
            Kind::Oneof(field)
        } else if let Some(field) = group::Field::new(&attrs, inferred_tag)? {
            Kind::Group(field)
        } else {
            bail!("no type attribute");
        };

        Ok(Some(Field { kind, debug_redact }))
    }

    /// Creates a new oneof `Field` from an iterator of field attributes.
    ///
    /// If the meta items are invalid, an error will be returned.
    /// If the field should be ignored, `None` is returned.
    pub fn new_oneof(attrs: Vec<Attribute>) -> Result<Option<Field>, Error> {
        let attrs = prost_attrs(attrs)?;
        let (attrs, debug_redact) = take_debug_redact(attrs)?;

        // TODO: check for ignore attribute.

        let kind = if let Some(field) = scalar::Field::new_oneof(&attrs)? {
            Kind::Scalar(field)
        } else if let Some(field) = message::Field::new_oneof(&attrs)? {
            Kind::Message(field)
        } else if let Some(field) = map::Field::new_oneof(&attrs)? {
            Kind::Map(field)
        } else if let Some(field) = group::Field::new_oneof(&attrs)? {
            Kind::Group(field)
        } else {
            bail!("no type attribute for oneof field");
        };

        Ok(Some(Field { kind, debug_redact }))
    }

    pub fn tags(&self) -> Vec<u32> {
        match self.kind {
            Kind::Scalar(ref scalar) => vec![scalar.tag],
            Kind::Message(ref message) => vec![message.tag],
            Kind::Map(ref map) => vec![map.tag],
            Kind::Oneof(ref oneof) => oneof.tags.clone(),
            Kind::Group(ref group) => vec![group.tag],
        }
    }

    /// Returns a statement which encodes the field.
    pub fn encode(&self, prost_path: &Path, ident: TokenStream) -> TokenStream {
        match self.kind {
            Kind::Scalar(ref scalar) => scalar.encode(prost_path, ident),
            Kind::Message(ref message) => message.encode(prost_path, ident),
            Kind::Map(ref map) => map.encode(prost_path, ident),
            Kind::Oneof(ref oneof) => oneof.encode(ident),
            Kind::Group(ref group) => group.encode(prost_path, ident),
        }
    }

    /// Returns an expression which evaluates to the result of merging a decoded
    /// value into the field.
    pub fn merge(&self, prost_path: &Path, ident: TokenStream) -> TokenStream {
        match self.kind {
            Kind::Scalar(ref scalar) => scalar.merge(prost_path, ident),
            Kind::Message(ref message) => message.merge(prost_path, ident),
            Kind::Map(ref map) => map.merge(prost_path, ident),
            Kind::Oneof(ref oneof) => oneof.merge(ident),
            Kind::Group(ref group) => group.merge(prost_path, ident),
        }
    }

    /// Returns an expression which evaluates to the encoded length of the field.
    pub fn encoded_len(&self, prost_path: &Path, ident: TokenStream) -> TokenStream {
        match self.kind {
            Kind::Scalar(ref scalar) => scalar.encoded_len(prost_path, ident),
            Kind::Map(ref map) => map.encoded_len(prost_path, ident),
            Kind::Message(ref msg) => msg.encoded_len(prost_path, ident),
            Kind::Oneof(ref oneof) => oneof.encoded_len(ident),
            Kind::Group(ref group) => group.encoded_len(prost_path, ident),
        }
    }

    /// Returns a statement which clears the field.
    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        match self.kind {
            Kind::Scalar(ref scalar) => scalar.clear(ident),
            Kind::Message(ref message) => message.clear(ident),
            Kind::Map(ref map) => map.clear(ident),
            Kind::Oneof(ref oneof) => oneof.clear(ident),
            Kind::Group(ref group) => group.clear(ident),
        }
    }

    pub fn default(&self, prost_path: &Path) -> TokenStream {
        match self.kind {
            Kind::Scalar(ref scalar) => scalar.default(prost_path),
            _ => quote!(::core::default::Default::default()),
        }
    }

    /// Produces the fragment implementing debug for the given field.
    pub fn debug(&self, prost_path: &Path, ident: TokenStream) -> TokenStream {
        if self.debug_redact {
            // Hide the whole value, including whether it is present, and how many elements it has.
            return quote! {
                {
                    // Avoids an unused variable warning when `ident` is a binding, as it is for a oneof.
                    let _ = &#ident;
                    struct RedactedWrapper;
                    impl ::core::fmt::Debug for RedactedWrapper {
                        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                            f.write_str("[REDACTED]")
                        }
                    }
                    RedactedWrapper
                }
            };
        }
        match self.kind {
            Kind::Scalar(ref scalar) => {
                let wrapper = scalar.debug(prost_path, quote!(ScalarWrapper));
                quote! {
                    {
                        #wrapper
                        ScalarWrapper(&#ident)
                    }
                }
            }
            Kind::Map(ref map) => {
                let wrapper = map.debug(prost_path, quote!(MapWrapper));
                quote! {
                    {
                        #wrapper
                        MapWrapper(&#ident)
                    }
                }
            }
            _ => quote!(&#ident),
        }
    }

    pub fn methods(&self, prost_path: &Path, ident: &TokenStream) -> Option<TokenStream> {
        match self.kind {
            Kind::Scalar(ref scalar) => scalar.methods(ident),
            Kind::Map(ref map) => map.methods(prost_path, ident),
            _ => None,
        }
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
    fn as_str(self) -> &'static str {
        match self {
            Label::Optional => "optional",
            Label::Required => "required",
            Label::Repeated => "repeated",
        }
    }

    fn variants() -> slice::Iter<'static, Label> {
        const VARIANTS: &[Label] = &[Label::Optional, Label::Required, Label::Repeated];
        VARIANTS.iter()
    }

    /// Parses a string into a field label.
    /// If the string doesn't match a field label, `None` is returned.
    fn from_attr(attr: &Meta) -> Option<Label> {
        if let Meta::Path(ref path) = *attr {
            for &label in Label::variants() {
                if path.is_ident(label.as_str()) {
                    return Some(label);
                }
            }
        }
        None
    }
}

impl fmt::Debug for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Get the items belonging to the 'prost' list attribute, e.g. `#[prost(foo, bar="baz")]`.
fn prost_attrs(attrs: Vec<Attribute>) -> Result<Vec<Meta>, Error> {
    let mut result = Vec::new();
    for attr in attrs.iter() {
        if let Meta::List(meta_list) = &attr.meta {
            if meta_list.path.is_ident("prost") {
                result.extend(
                    meta_list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?,
                )
            }
        }
    }
    Ok(result)
}

/// Removes any `debug_redact` items from the field's attributes, and returns the remaining items
/// along with whether the field should be redacted.
///
/// Every kind of field accepts `debug_redact`, so it is handled here rather than by each kind's
/// parser, which would reject it as an unknown attribute.
fn take_debug_redact(attrs: Vec<Meta>) -> Result<(Vec<Meta>, bool), Error> {
    let mut debug_redact = None;
    let mut remaining = Vec::with_capacity(attrs.len());
    for attr in attrs {
        if let Some(value) = bool_attr("debug_redact", &attr)? {
            set_option(
                &mut debug_redact,
                value,
                "duplicate debug_redact attributes",
            )?;
        } else {
            remaining.push(attr);
        }
    }
    Ok((remaining, debug_redact.unwrap_or(false)))
}

pub fn set_option<T>(option: &mut Option<T>, value: T, message: &str) -> Result<(), Error>
where
    T: fmt::Debug,
{
    if let Some(ref existing) = *option {
        bail!("{message}: {existing:?} and {value:?}");
    }
    *option = Some(value);
    Ok(())
}

pub fn set_bool(b: &mut bool, message: &str) -> Result<(), Error> {
    if *b {
        bail!("{message}");
    } else {
        *b = true;
        Ok(())
    }
}

/// Unpacks an attribute into a (key, boolean) pair, returning the boolean value.
/// If the key doesn't match the attribute, `None` is returned.
fn bool_attr(key: &str, attr: &Meta) -> Result<Option<bool>, Error> {
    if !attr.path().is_ident(key) {
        return Ok(None);
    }
    match *attr {
        Meta::Path(..) => Ok(Some(true)),
        Meta::List(ref meta_list) => Ok(Some(meta_list.parse_args::<LitBool>()?.value())),
        Meta::NameValue(MetaNameValue {
            value:
                Expr::Lit(ExprLit {
                    lit: Lit::Str(ref lit),
                    ..
                }),
            ..
        }) => lit
            .value()
            .parse::<bool>()
            .map_err(Error::from)
            .map(Option::Some),
        Meta::NameValue(MetaNameValue {
            value:
                Expr::Lit(ExprLit {
                    lit: Lit::Bool(LitBool { value, .. }),
                    ..
                }),
            ..
        }) => Ok(Some(value)),
        _ => bail!("invalid {key} attribute"),
    }
}

/// Checks if an attribute matches a word.
fn word_attr(key: &str, attr: &Meta) -> bool {
    if let Meta::Path(ref path) = *attr {
        path.is_ident(key)
    } else {
        false
    }
}

pub(super) fn tag_attr(attr: &Meta) -> Result<Option<u32>, Error> {
    if !attr.path().is_ident("tag") {
        return Ok(None);
    }
    match *attr {
        Meta::List(ref meta_list) => Ok(Some(meta_list.parse_args::<LitInt>()?.base10_parse()?)),
        Meta::NameValue(MetaNameValue {
            value: Expr::Lit(ref expr),
            ..
        }) => match expr.lit {
            Lit::Str(ref lit) => lit
                .value()
                .parse::<u32>()
                .map_err(Error::from)
                .map(Option::Some),
            Lit::Int(ref lit) => Ok(Some(lit.base10_parse()?)),
            _ => bail!("invalid tag attribute: {attr:?}"),
        },
        _ => bail!("invalid tag attribute: {attr:?}"),
    }
}

fn tags_attr(attr: &Meta) -> Result<Option<Vec<u32>>, Error> {
    if !attr.path().is_ident("tags") {
        return Ok(None);
    }
    match *attr {
        Meta::List(ref meta_list) => Ok(Some(
            meta_list
                .parse_args_with(Punctuated::<LitInt, Token![,]>::parse_terminated)?
                .iter()
                .map(LitInt::base10_parse)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        Meta::NameValue(MetaNameValue {
            value:
                Expr::Lit(ExprLit {
                    lit: Lit::Str(ref lit),
                    ..
                }),
            ..
        }) => lit
            .value()
            .split(',')
            .map(|s| s.trim().parse::<u32>().map_err(Error::from))
            .collect::<Result<Vec<u32>, _>>()
            .map(Some),
        _ => bail!("invalid tag attribute: {attr:?}"),
    }
}
