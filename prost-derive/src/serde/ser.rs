use anyhow::{anyhow, Error};
use heck::{ToShoutySnakeCase, ToSnakeCase};
use proc_macro2::{Ident, TokenStream};
use quote::{quote, TokenStreamExt};
use syn::{Expr, Generics};

use crate::{
    field::{self, Field, Json},
    serde::utils::ToProtoCamelCase,
};

pub fn impl_for_message(
    struct_ident: &Ident,
    generics: &Generics,
    fields: &[(TokenStream, Field)],
) -> Result<TokenStream, Error> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let struct_ident_str = struct_ident.to_string();

    let num_fields = fields.len();

    let mut ser_stmts = TokenStream::new();

    for (field_ident, field) in fields {
        use field::scalar::Kind;

        if let Some(json) = field.json() {
            // Only scalar, message, group and map fields may have the json attribute.

            let json_field_name = match json {
                Some(Json {
                    json_name: Some(name),
                    ..
                }) => name.to_owned(),
                Some(Json {
                    proto_name: Some(name),
                    ..
                }) => name.to_proto_camel_case(),
                Some(_) | None => field_ident.to_string().to_proto_camel_case(),
            };

            // Map a group field to an equivalent message field because they share the same
            // serialization impl.
            let remapped_group_field;
            let field = if let Field::Group(group) = field {
                remapped_group_field = group.to_message_field();
                &remapped_group_field
            } else {
                field
            };

            match field {
                Field::Scalar(scalar) => {
                    let wrapper = wrapper_for_ty(&scalar.ty);

                    match &scalar.kind {
                        Kind::Plain(_) => {
                            ser_stmts.append_all(quote! {
                                if __config.emit_fields_with_default_value
                                    || !_private::is_default_value(&__self.#field_ident)
                                {
                                    _serde::ser::SerializeStruct::serialize_field(
                                        &mut __serde_state,
                                        #json_field_name,
                                        &_private::SerWithConfig(
                                            #wrapper(&__self.#field_ident),
                                            __config,
                                        )
                                    )?;
                                }
                            });
                        }
                        Kind::Required(_) => {
                            ser_stmts.append_all(quote! {
                                _serde::ser::SerializeStruct::serialize_field(
                                    &mut __serde_state,
                                    #json_field_name,
                                    &_private::SerWithConfig(
                                        #wrapper(&__self.#field_ident),
                                        __config,
                                    )
                                )?;
                            });
                        }
                        Kind::Optional(_) => {
                            ser_stmts.append_all(quote! {
                                if let _private::Option::Some(val) = &__self.#field_ident {
                                    _serde::ser::SerializeStruct::serialize_field(
                                        &mut __serde_state,
                                        #json_field_name,
                                        &_private::SerWithConfig(
                                            #wrapper(val),
                                            __config,
                                        )
                                    )?;
                                } else {
                                    if __config.emit_nulled_optional_fields {
                                        _serde::ser::SerializeStruct::serialize_field(
                                            &mut __serde_state,
                                            #json_field_name,
                                            &_private::Option::<()>::None
                                        )?;
                                    }
                                }
                            });
                        }
                        Kind::Repeated | Kind::Packed => {
                            ser_stmts.append_all(quote! {
                                if __config.emit_fields_with_default_value
                                    || !_private::is_default_value(&__self.#field_ident)
                                {
                                    _serde::ser::SerializeStruct::serialize_field(
                                        &mut __serde_state,
                                        #json_field_name,
                                        &_private::SerWithConfig(
                                            _private::SerMappedVecItems(
                                                &__self.#field_ident,
                                                #wrapper
                                            ),
                                            __config,
                                        )
                                    )?;
                                }
                            });
                        }
                    }
                }
                Field::Message(message) => {
                    use field::Label;

                    match message.label {
                        Label::Required => {
                            ser_stmts.append_all(quote! {
                                _serde::ser::SerializeStruct::serialize_field(
                                    &mut __serde_state,
                                    #json_field_name,
                                    &_private::SerWithConfig(&__self.#field_ident, __config)
                                )?;
                            });
                        }
                        Label::Optional => {
                            ser_stmts.append_all(quote! {
                                if let _private::Option::Some(__val) = &__self.#field_ident {
                                    _serde::ser::SerializeStruct::serialize_field(
                                        &mut __serde_state,
                                        #json_field_name,
                                        &_private::SerWithConfig(__val, __config)
                                    )?;
                                } else {
                                    if __config.emit_nulled_optional_fields {
                                        _serde::ser::SerializeStruct::serialize_field(
                                            &mut __serde_state,
                                            #json_field_name,
                                            &_private::Option::<()>::None
                                        )?;
                                    }
                                }
                            });
                        }
                        Label::Repeated => {
                            ser_stmts.append_all(quote! {
                                if __config.emit_fields_with_default_value
                                    || !_private::is_default_value(&__self.#field_ident)
                                {
                                    _serde::ser::SerializeStruct::serialize_field(
                                        &mut __serde_state,
                                        #json_field_name,
                                        &_private::SerWithConfig(&__self.#field_ident, __config)
                                    )?;
                                }
                            });
                        }
                    }
                }
                Field::Map(map) => {
                    use field::map::ValueTy;

                    let wrapper = match &map.value_ty {
                        ValueTy::Scalar(ty) => wrapper_for_ty(ty),
                        ValueTy::Message => quote! { _private::SerIdentity },
                    };

                    ser_stmts.append_all(quote! {
                        if __config.emit_fields_with_default_value
                            || !_private::is_default_value(&__self.#field_ident)
                        {
                            _serde::ser::SerializeStruct::serialize_field(
                                &mut __serde_state,
                                #json_field_name,
                                &_private::SerWithConfig(
                                    _private::SerMappedMapItems(&__self.#field_ident, #wrapper),
                                    __config,
                                )
                            )?;
                        }
                    });
                }
                Field::Group(_) => {
                    // We should've replaced the group field with an equivalant message field.
                    unreachable!();
                }
                Field::Oneof(_) => unreachable!(),
            }
        } else {
            // Must be an oneof field.
            let Field::Oneof(oneof) = field else {
                unreachable!()
            };

            let oneof_ty = &oneof.ty;
            ser_stmts.append_all(quote! {
                if let _private::Option::Some(val) = &__self.#field_ident {
                    <#oneof_ty as _private::SerializeOneOf>::serialize_oneof(
                        val,
                        &mut __serde_state,
                        __config,
                    )?;
                }
            });
        }
    }

    Ok(quote! {
        impl #impl_generics _private::CustomSerialize for #struct_ident #ty_generics
        #where_clause
        {
            fn serialize<__S>(
                &self,
                __serializer: __S,
                __config: &_private::SerializerConfig,
            ) -> _private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let __self = self;

                let mut __serde_state = _serde::Serializer::serialize_struct(
                    __serializer,
                    #struct_ident_str,
                    #num_fields,
                )?;

                #ser_stmts

                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }

        impl #impl_generics _serde::Serialize for #struct_ident #ty_generics
        #where_clause
        {
            #[inline]
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let __config = <_private::SerializerConfig as _private::Default>::default();
                _private::CustomSerialize::serialize(self, __serializer, &__config)
            }
        }
    })
}

pub fn impl_for_oneof(
    oneof_ident: &Ident,
    generics: &Generics,
    fields: &[(Ident, Field)],
) -> Result<TokenStream, Error> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let ser_match_variants = fields
        .iter()
        .map(|(field_ident, field)| {
            let json_field_name = match &field.json().unwrap() {
                Some(Json {
                    json_name: Some(name),
                    ..
                }) => name.to_owned(),
                Some(Json {
                    proto_name: Some(name),
                    ..
                }) => name.to_proto_camel_case(),
                Some(_) | None => field_ident
                    .to_string()
                    .to_snake_case()
                    .to_proto_camel_case(),
            };

            // Map a group field to an equivalent message field because they share the same
            // serialization impl.
            let remapped_group_field;
            let field = if let Field::Group(group) = field {
                remapped_group_field = group.to_message_field();
                &remapped_group_field
            } else {
                field
            };

            let arm = match field {
                Field::Scalar(scalar) => {
                    let wrapper = wrapper_for_ty(&scalar.ty);
                    quote! {
                        Self::#field_ident(val) => __serializer.serialize_field(
                            #json_field_name,
                            &_private::SerWithConfig(
                                #wrapper(val),
                                __config,
                            ),
                        )
                    }
                }
                Field::Message(_) => {
                    quote! {
                        Self::#field_ident(__val) => __serializer.serialize_field(
                            #json_field_name,
                            &_private::SerWithConfig(
                                __val,
                                __config,
                            ),
                        )
                    }
                }
                Field::Group(_) => unreachable!(),
                Field::Map(_) => return Err(anyhow!("unsupported map field inside oneof")),
                Field::Oneof(_) => return Err(anyhow!("unsupported oneof field inside oneof")),
            };

            Ok(arm)
        })
        .collect::<Result<Vec<_>, Error>>()?;

    Ok(quote! {
        impl #impl_generics _private::SerializeOneOf for #oneof_ident #ty_generics
        #where_clause
        {
            fn serialize_oneof<__S>(
                &self,
                __serializer: &mut __S,
                __config: &_private::SerializerConfig,
            ) -> _private::Result<(), __S::Error>
            where
                __S: _serde::ser::SerializeStruct,
            {
                match self {
                    #(#ser_match_variants,)*
                }
            }
        }
    })
}

pub fn impl_for_enum(
    enum_ident: &Ident,
    generics: &Generics,
    variants: &[(Ident, Expr, Option<Json>)],
) -> Result<TokenStream, Error> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let match_arms = variants
        .iter()
        .map(|(variant_ident, discr, json)| {
            let json_value = match json {
                Some(Json {
                    proto_name: Some(proto_name),
                    ..
                }) => proto_name.to_owned(),
                _ => format!("{enum_ident}_{variant_ident}").to_shouty_snake_case(),
            };

            quote! {
                Self::#variant_ident => {
                    if __config.emit_enum_values_as_integer {
                        __serializer.serialize_i32(#discr)
                    } else {
                        __serializer.serialize_str(#json_value)
                    }
                }
            }
        })
        .collect::<Vec<_>>();

    Ok(quote! {
        impl #impl_generics _private::CustomSerialize for #enum_ident #ty_generics
        #where_clause
        {
            fn serialize<__S>(
                &self,
                __serializer: __S,
                __config: &_private::SerializerConfig,
            ) -> _private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                match self {
                    #(#match_arms,)*
                }
            }
        }

        impl #impl_generics _serde::Serialize for #enum_ident #ty_generics
        #where_clause
        {
            #[inline]
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let __config = <_private::SerializerConfig as _private::Default>::default();
                _private::CustomSerialize::serialize(self, __serializer, &__config)
            }
        }
    })
}

fn wrapper_for_ty(ty: &field::scalar::Ty) -> TokenStream {
    use field::scalar::Ty;
    match ty {
        Ty::Int32
        | Ty::Uint32
        | Ty::Sint32
        | Ty::Fixed32
        | Ty::Sfixed32
        | Ty::String
        | Ty::Bool => {
            quote! { _private::SerSerde }
        }
        Ty::Int64 | Ty::Uint64 | Ty::Sint64 | Ty::Fixed64 | Ty::Sfixed64 => {
            quote! { _private::SerAsDisplay }
        }
        Ty::Bytes(_) => {
            quote! { _private::SerBytesAsBase64 }
        }
        Ty::Float => {
            quote! { _private::SerFloat32 }
        }
        Ty::Double => {
            quote! { _private::SerFloat64 }
        }
        Ty::Enumeration(path) => {
            quote! { _private::SerEnum::<#path>::new }
        }
    }
}
