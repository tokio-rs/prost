use anyhow::{bail, Error};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, ExprLit, Lit, Meta, MetaNameValue, Path};

use crate::field::{set_bool, set_option, word_attr};

#[derive(Clone)]
pub struct Field {
    pub default_fn: Option<Path>,
}

impl Field {
    pub fn new(attrs: &[Meta]) -> Result<Option<Field>, Error> {
        let mut skip = false;
        let mut default_fn = None;
        let mut default_lit = None;
        let mut unknown_attrs = Vec::new();

        for attr in attrs {
            if word_attr("skip", attr) {
                set_bool(&mut skip, "duplicate skip attribute")?;
            } else if let Meta::NameValue(MetaNameValue { path, value, .. }) = attr {
                if path.is_ident("default") {
                    match value {
                        // There has to be a better way...
                        Expr::Lit(ExprLit {
                            lit: Lit::Str(lit), ..
                        }) => set_option(&mut default_lit, lit, "duplicate default attributes")?,
                        _ => bail!("default attribute value must be a string literal"),
                    };
                } else {
                    unknown_attrs.push(attr);
                }
            } else {
                unknown_attrs.push(attr);
            }
        }

        if !skip {
            return Ok(None);
        }

        if !unknown_attrs.is_empty() {
            bail!(
                "unknown attribute(s) for skipped field: #[prost({})]",
                quote!(#(#unknown_attrs),*)
            );
        }

        if let Some(lit) = default_lit {
            let fn_path: Path = syn::parse_str(&lit.value())
                .map_err(|_| anyhow::anyhow!("invalid path for default function"))?;
            if default_fn.is_some() {
                bail!("duplicate default attribute for skipped field");
            }
            default_fn = Some(fn_path);
        }

        Ok(Some(Field { default_fn }))
    }

    pub fn clear(&self, ident: TokenStream) -> TokenStream {
        let default = self.default_value();
        quote!( #ident = #default; )
    }

    pub fn default_value(&self) -> TokenStream {
        if let Some(ref path) = self.default_fn {
            quote! { #path() }
        } else {
            quote! { ::core::default::Default::default() }
        }
    }
}
