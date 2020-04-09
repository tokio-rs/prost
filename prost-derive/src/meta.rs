use itertools::Itertools;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse::ParseStream;
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, DeriveInput, Expr, ExprAssign, Ident, Result, Token};

pub struct MetaAttrs(Vec<ExprAssign>);

pub fn try_meta(attr: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let input = parse_macro_input::parse::<DeriveInput>(input)?;
    let attributes = parse_macro_input::parse::<MetaAttrs>(attr)?;
    struct_and_meta_impl(attributes, input).map(TokenStream::from)
}

fn struct_and_meta_impl(
    attr: MetaAttrs,
    input: DeriveInput,
) -> syn::Result<proc_macro2::TokenStream> {
    let meta_impl = meta_impl(attr)?;
    let ident = &input.ident;
    let contents = quote! {
        #input

        impl #ident {
            #(#meta_impl)*
        }

        impl ::prost::MessageNamed for #ident {
            fn fqname() -> &'static str {
                Self::fqname()
            }
        }
    };

    Ok(contents.into())
}

impl syn::parse::Parse for MetaAttrs {
    fn parse(input: ParseStream) -> Result<Self> {
        let expressions = Punctuated::<Expr, Token![,]>::parse_terminated(input)?;
        Ok(Self(
            expressions
                .into_iter()
                .flat_map(|expr| match expr {
                    syn::Expr::Assign(e) => Ok(e),
                    _ => Err(syn::Error::new_spanned(
                        expr,
                        "meta attributes must be in the `key=value` form",
                    )),
                })
                .collect(),
        ))
    }
}

struct Getter {
    name: syn::Ident,
    return_value: proc_macro2::TokenStream,
    return_type: proc_macro2::TokenStream,
}

impl Getter {
    fn from_expr(expr: &syn::ExprAssign) -> syn::Result<Self> {
        use syn::spanned::Spanned;

        let (return_value, return_type) = value_and_return_type(&*expr.right)?;
        let expr_left = &expr.left;
        Ok(Self {
            name: Ident::new(&*quote! { #expr_left }.to_string(), expr.left.span()),
            return_value,
            return_type,
        })
    }
}

fn value_and_return_type(
    expr: &syn::Expr,
) -> syn::Result<(proc_macro2::TokenStream, proc_macro2::TokenStream)> {
    if let syn::Expr::Lit(l) = expr {
        let (return_type, return_value) = match &l.lit {
            syn::Lit::Str(lit_str) => (quote! { &'static str }, quote! { #lit_str }),
            syn::Lit::ByteStr(byte_str) => {
                let vec_contents = byte_str.value().iter().map(|bit| bit.to_string()).join(",");
                (quote! { &'static [u8] }, quote! { [#vec_contents] })
            }
            syn::Lit::Byte(byte_lit) => {
                let byte = byte_lit.value();
                let value = quote! { #byte };
                (quote! { u8 }, value)
            }
            syn::Lit::Char(char_lit) => {
                let c = char_lit.value();
                let value = quote! { #c };
                (quote! { char }, value)
            }
            syn::Lit::Int(lit_int) => (quote! { i64 }, quote! { #lit_int }), // is this a good choice ?
            syn::Lit::Float(lit_float) => {
                let float_value = lit_float.base10_digits();
                let value = quote! { #float_value };
                (quote! { f64 }, value)
            } // is this a good choice ?
            syn::Lit::Bool(lit_bool) => (quote! { bool }, quote! { #lit_bool }),
            _ => {
                return Err(syn::Error::new_spanned(
                    l,
                    "values of this type are not supported\n{:?}",
                ))
            }
        };
        Ok((return_value, return_type))
    } else {
        Err(syn::Error::new_spanned(
            expr,
            "expected litteral, found\n{:?}",
        ))
    }
}

fn meta_impl(attrs: MetaAttrs) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let getter_functions = attrs
        .0
        .iter()
        .flat_map(|expr| {
            Getter::from_expr(expr).map(|g| {
                let Getter {
                    name,
                    return_type,
                    return_value,
                } = g;
                Ok(quote! {
                    const fn #name() -> #return_type {
                        #return_value
                    }
                })
            })
        })
        .collect::<syn::Result<Vec<proc_macro2::TokenStream>>>()?;

    Ok(getter_functions)
}
