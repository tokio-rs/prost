use anyhow::Error;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Expr, Generics};

use crate::field::{Field, Json};

mod de;
mod ser;
mod utils;

pub fn impls_for_enum(
    enum_ident: &Ident,
    generics: &Generics,
    variants: &[(Ident, Expr, Option<Json>)],
) -> Result<TokenStream, Error> {
    let serialize_impl = ser::impl_for_enum(enum_ident, generics, variants)?;
    let deserialize_impl = de::impl_for_enum(enum_ident, generics, variants)?;

    let items = quote! {
        extern crate prost as _prost;

        use _prost::serde::private::_serde;
        use _prost::serde::private as _private;

        #serialize_impl

        #deserialize_impl
    };

    let wrapped = quote! {
        #[doc(hidden)]
        const _: () = {
            #items
        };
    };

    Ok(wrapped)
}

pub fn impls_for_oneof(
    ident: &Ident,
    generics: &Generics,
    fields: &[(Ident, Field)],
) -> Result<TokenStream, Error> {
    let serialize_impl = ser::impl_for_oneof(ident, generics, fields)?;
    let deserialize_impl = de::impl_for_oneof(ident, generics, fields)?;

    let items = quote! {
        extern crate prost as _prost;

        use _prost::serde::private::_serde;
        use _prost::serde::private as _private;

        #serialize_impl

        #deserialize_impl
    };

    let wrapped = quote! {
        #[doc(hidden)]
        const _: () = {
            #items
        };
    };

    Ok(wrapped)
}

pub fn impls_for_struct(
    struct_ident: &Ident,
    generics: &Generics,
    fields: &[(TokenStream, Field)],
) -> Result<TokenStream, Error> {
    let serialize_impl = ser::impl_for_message(struct_ident, generics, fields)?;
    let deserialize_impl = de::impl_for_message(struct_ident, generics, fields)?;

    let items = quote! {
        extern crate prost as _prost;

        use _prost::serde::private::_serde;
        use _prost::serde::private as _private;

        #serialize_impl

        #deserialize_impl
    };

    let wrapped = quote! {
        #[doc(hidden)]
        const _: () = {
            #items
        };
    };

    Ok(wrapped)
}
