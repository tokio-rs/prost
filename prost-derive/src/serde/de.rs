use std::iter;

use anyhow::{anyhow, Error};
use heck::{ToShoutySnakeCase, ToSnakeCase};
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Expr, Generics};

use crate::{
    field::{self, scalar, Field, Json},
    serde::utils::ToProtoCamelCase,
};

pub fn impl_for_message(
    struct_ident: &Ident,
    _generics: &Generics,
    fields: &[(TokenStream, Field)],
) -> Result<TokenStream, Error> {
    let full_struct_name = format!("struct {}", struct_ident);

    let mut field_vals = vec![];
    let mut field_assignments = vec![];
    let mut field_variants = vec![];
    let mut field_match_arms = vec![];
    let mut field_match_oneofs = vec![];
    let mut field_matches = vec![];
    let mut field_required_checks = vec![];

    for (field_idx, (field_ident, field)) in fields.iter().enumerate() {
        let field_ident_str = field_ident.to_string();

        let field_variant_ident = format_ident!("__field{}", field_idx);

        field_vals.push(quote! { #field_variant_ident });
        field_assignments.push(quote! {
            #field_ident: _private::Option::unwrap_or_default(#field_variant_ident)
        });

        if let Field::Oneof(oneof) = field {
            let ty_path = &oneof.ty;
            field_variants.push(quote! {
                #field_variant_ident(
                    <#ty_path as _private::DeserializeOneOf>::FieldKey
                )
            })
        } else {
            field_variants.push(quote! { #field_variant_ident })
        }

        if let Some(json) = field.json() {
            // Only a scalar, message, group or map field may have the json attribute.

            let proto_field_name = match json {
                Some(Json {
                    proto_name: Some(proto_name),
                    ..
                }) => proto_name,
                _ => &field_ident_str,
            };

            let json_field_name = match json {
                Some(Json {
                    json_name: Some(json_name),
                    ..
                }) => json_name.to_owned(),
                Some(Json {
                    proto_name: Some(proto_name),
                    ..
                }) => proto_name.to_proto_camel_case(),
                Some(_) | None => field_ident_str.to_proto_camel_case(),
            };

            if proto_field_name != &json_field_name {
                field_match_arms.push(quote! {
                    #proto_field_name | #json_field_name
                        => _private::Ok(__Field::#field_variant_ident)
                });
            } else {
                field_match_arms.push(quote! {
                    #proto_field_name
                        => _private::Ok(__Field::#field_variant_ident)
                });
            }

            let deserializer = deserializer_for_field(field)?;
            field_matches.push(quote! {
                __Field::#field_variant_ident => {
                    if _private::Option::is_some(&#field_variant_ident) {
                        return _private::Err(
                            <__A::Error as _serde::de::Error>::duplicate_field(#field_ident_str)
                        );
                    }
                    let val =_serde::de::MapAccess::next_value_seed(
                        &mut __map,
                        _private::DesIntoWithConfig::<#deserializer, _>::new(__config)
                    )?;
                    #field_variant_ident = _private::Some(val);
                }
            });

            if field.is_required() {
                field_required_checks.push(quote! {
                    if #field_variant_ident.is_none() {
                        return _private::Err(
                            <__A::Error as _serde::de::Error>::missing_field(#field_ident_str)
                        );
                    }
                });
            }
        }

        if let Field::Oneof(oneof) = field {
            let ty_path = &oneof.ty;

            field_match_oneofs.push(quote! {
                if let _private::Some(field_key)
                    = <#ty_path as _private::DeserializeOneOf>::deserialize_field_key(__value)
                {
                    return _private::Ok(__Field::#field_variant_ident(field_key));
                }
            });

            field_matches.push(quote! {
                __Field::#field_variant_ident(key) => {
                    if _private::Option::is_some(&#field_variant_ident) {
                        return _private::Err(
                            <__A::Error as _serde::de::Error>::duplicate_field(#field_ident_str)
                        );
                    }
                    let __val = _serde::de::MapAccess::next_value_seed(
                        &mut __map,
                        _private::OneOfDeserializer(key, __config),
                    )?;
                    if __val.is_some() {
                        #field_variant_ident = _private::Some(__val);
                    }
                }
            })
        }
    }

    let map_field = quote! {
        enum __Field {
            #(#field_variants,)*
            __unknown,
        }

        struct __FieldVisitor<'a>(&'a _private::DeserializerConfig);

        impl<'a, 'de> _serde::de::Visitor<'de> for __FieldVisitor<'a> {
            type Value = __Field;

            fn expecting(
                &self,
                __formatter: &mut _private::fmt::Formatter
            ) -> _serde::__private::fmt::Result {
                _private::fmt::Formatter::write_str(__formatter, "field identifier")
            }

            fn visit_str<__E>(self, __value: &str) -> _private::Result<Self::Value, __E>
            where
                __E: _serde::de::Error
            {
                let __config = self.0;

                #(#field_match_oneofs)*

                match __value {
                    #(#field_match_arms,)*
                    _ => {
                        if __config.ignore_unknown_fields {
                            _private::Ok(__Field::__unknown)
                        } else {
                            _private::Err(<__E as _serde::de::Error>::unknown_field(__value, &[]))
                        }
                    },
                }
            }
        }

        impl<'de> _private::CustomDeserialize<'de> for __Field {
            fn deserialize<__D>(
                __deserializer: __D,
                __config: &_private::DeserializerConfig
            ) -> _private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                _serde::Deserializer::deserialize_identifier(
                    __deserializer,
                    __FieldVisitor(__config),
                )
            }
        }
    };

    let map_visitor = quote! {
        struct __Visitor<'a>(&'a _private::DeserializerConfig);

        impl<'a, 'de> _serde::de::Visitor<'de> for __Visitor<'a> {
            type Value = #struct_ident;

            fn expecting(&self, __formatter: &mut _private::fmt::Formatter) -> _private::fmt::Result {
                _private::fmt::Formatter::write_str(__formatter, #full_struct_name)
            }

            fn visit_map<__A>(self, mut __map: __A) -> _private::Result<Self::Value, __A::Error>
            where
                __A: _serde::de::MapAccess<'de>
            {
                let __config = self.0;

                #(let mut #field_vals = _private::None;)*

                while let _private::Some(__key)
                    = _serde::de::MapAccess::next_key_seed(
                        &mut __map,
                        _private::DesWithConfig::<__Field>::new(__config)
                    )?
                {
                    match __key {
                        #(#field_matches,)*
                        __Field::__unknown => {
                            _serde::de::MapAccess::next_value::<_serde::de::IgnoredAny>(
                                &mut __map
                            )?;
                        }
                    }
                }

                #(#field_required_checks)*

                _private::Ok(#struct_ident {
                    #(#field_assignments),*
                })
            }
        }
    };

    Ok(quote! {
        impl<'de> _private::CustomDeserialize<'de> for #struct_ident {
            fn deserialize<__D>(
                __deserializer: __D,
                __config: &_private::DeserializerConfig
            ) -> _private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #map_field

                #map_visitor

                _serde::Deserializer::deserialize_map(
                    __deserializer,
                    __Visitor(__config),
                )
            }
        }

        impl<'de> _serde::Deserialize<'de> for #struct_ident {
            #[inline]
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                let __config = <_private::DeserializerConfig as _private::Default>::default();
                <Self as _private::CustomDeserialize>::deserialize(
                    __deserializer,
                    &__config,
                )
            }
        }
    })
}

pub fn impl_for_oneof(
    oneof_ident: &Ident,
    _generics: &Generics,
    fields: &[(Ident, Field)],
) -> Result<TokenStream, Error> {
    let mut field_keys = vec![];
    let mut match_field_key_str_arms = vec![];
    let mut match_field_key_arms = vec![];

    let field_key_enum_ident = format_ident!("{}FieldKey", oneof_ident);

    for (field_idx, (field_ident, field)) in fields.iter().enumerate() {
        let field_key_ident = format_ident!("__field{}", field_idx);
        let field_ident_str = field_ident.to_string();

        let Some(json) = field.json() else {
            return Err(anyhow!("unsupported field in oneof"));
        };

        let proto_field_name = match json {
            Some(Json {
                proto_name: Some(proto_name),
                ..
            }) => proto_name.to_owned(),
            _ => field_ident_str.to_snake_case(),
        };

        let json_field_name = match json {
            Some(Json {
                json_name: Some(json_name),
                ..
            }) => json_name.to_owned(),
            Some(Json {
                proto_name: Some(proto_name),
                ..
            }) => proto_name.to_proto_camel_case(),
            Some(_) | None => field_ident_str.to_snake_case().to_proto_camel_case(),
        };

        if proto_field_name != json_field_name {
            match_field_key_str_arms.push(quote! {
                #proto_field_name | #json_field_name
                    => _private::Some(#field_key_enum_ident::#field_key_ident)
            });
        } else {
            match_field_key_str_arms.push(quote! {
                #proto_field_name
                    => _private::Some(#field_key_enum_ident::#field_key_ident)
            });
        }

        assert!(field.is_required());

        let deserializer = deserializer_for_field(field)?;
        match_field_key_arms.push(quote! {
            #field_key_enum_ident::#field_key_ident => {
                let __val = <
                    _private::OptionDeserializer<#deserializer>
                        as _private::DeserializeInto<_private::Option<_>>
                >::deserialize_into(__deserializer, __config)?;
                _private::Ok(__val.map(Self::#field_ident))
            }
        });

        field_keys.push(field_key_ident);
    }

    Ok(quote! {
        pub enum #field_key_enum_ident {
            #(#field_keys,)*
        }

        impl _private::DeserializeOneOf for #oneof_ident {
            type FieldKey = #field_key_enum_ident;

            fn deserialize_field_key(__val: &str) -> _private::Option<Self::FieldKey> {
                match __val {
                    #(#match_field_key_str_arms,)*
                    _ => _private::None,
                }
            }

            fn deserialize_by_field_key<'de, __D>(
                __field_key: Self::FieldKey,
                __deserializer: __D,
                __config: &_private::DeserializerConfig,
            ) -> _private::Result<_private::Option<Self>, __D::Error>
            where
                __D: _serde::de::Deserializer<'de>
            {
                match __field_key {
                    #(#match_field_key_arms,)*
                }
            }
        }
    })
}

pub fn impl_for_enum(
    enum_ident: &Ident,
    _generics: &Generics,
    variants: &[(Ident, Expr, Option<Json>)],
) -> Result<TokenStream, Error> {
    let invalid_val_err_msg = format!("a valid enum value (`{}`)", enum_ident);

    let (str_arms, int_arms): (Vec<_>, Vec<_>) = variants
        .iter()
        .map(|(variant_ident, descr, json)| {
            let json_value = match json {
                Some(Json {
                    proto_name: Some(proto_name),
                    proto_alt_names,
                    ..
                }) => iter::once(proto_name.to_owned())
                    .chain(proto_alt_names.iter().cloned())
                    .collect::<Vec<_>>(),
                _ => vec![format!("{enum_ident}_{variant_ident}").to_shouty_snake_case()],
            };
            let str_arm = quote! {
                #(#json_value)|* => _private::Ok(_private::Ok(Self::#variant_ident))
            };
            let int_arm = quote! {
                #descr => _private::Ok(_private::Ok(Self::#variant_ident))
            };
            (str_arm, int_arm)
        })
        .multiunzip();

    Ok(quote! {
        impl _private::DeserializeEnum for #enum_ident {
            fn deserialize_from_i32<__E>(val: i32)
                -> _private::Result<_private::Result<Self, i32>, __E>
            where
                __E: _serde::de::Error
            {
                match val {
                    #(#int_arms,)*
                    _ => _private::Ok(_private::Err(val)),
                }
            }

            fn deserialize_from_str<__E>(val: &str)
                -> _private::Result<_private::Result<Self, i32>, __E>
            where
                __E: _serde::de::Error
            {
                match val {
                    #(#str_arms,)*
                    _ => {
                        _private::Err(<__E as _serde::de::Error>::invalid_value(
                            _serde::de::Unexpected::Str(val),
                            &#invalid_val_err_msg
                        ))
                    }
                }
            }
        }
    })
}

fn deserializer_for_field(field: &Field) -> Result<TokenStream, Error> {
    // Map group fields to message fields, since they deserialize the same.
    let remapped_group_field;
    let field = if let Field::Group(group) = field {
        remapped_group_field = group.to_message_field();
        &remapped_group_field
    } else {
        field
    };
    Ok(match field {
        Field::Scalar(scalar) => {
            let de = deserializer_for_ty(&scalar.ty, false);
            match scalar.kind {
                scalar::Kind::Required(_) => de,
                scalar::Kind::Plain(_) => quote! { _private::DefaultDeserializer<#de> },
                scalar::Kind::Optional(_) => quote! { _private::OptionDeserializer<#de> },
                scalar::Kind::Repeated | scalar::Kind::Packed => {
                    quote! { _private::DefaultDeserializer<_private::VecDeserializer<#de>> }
                }
            }
        }
        Field::Message(message) => {
            let inner = if message.is_well_known_ty {
                quote! { _private::WellKnownDeserializer<::prost_types::serde::DesWellKnown<_>> }
            } else {
                quote! { _private::MessageDeserializer }
            };
            match message.label {
                field::Label::Optional => quote! {
                    _private::OptionDeserializer<#inner>
                },
                field::Label::Repeated => quote! {
                    _private::DefaultDeserializer<_private::VecDeserializer<#inner>>
                },
                field::Label::Required => inner,
            }
        }
        Field::Map(map) => {
            let key_deserializer = deserializer_for_ty(&map.key_ty, true);
            let val_deserializer = match &map.value_ty {
                field::map::ValueTy::Scalar(ty) => deserializer_for_ty(ty, false),
                field::map::ValueTy::Message => {
                    if map.is_value_well_known_ty {
                        quote! {
                            _private::DefaultDeserializer<_private::WellKnownDeserializer<
                                ::prost_types::serde::DesWellKnown<_>
                            >>
                        }
                    } else {
                        quote! { _private::DefaultDeserializer<_private::MessageDeserializer> }
                    }
                }
            };
            quote! {
                _private::DefaultDeserializer<
                    _private::MapDeserializer<#key_deserializer, #val_deserializer>
                >
            }
        }
        Field::Group(_) | Field::Oneof(_) => unreachable!(),
    })
}

fn deserializer_for_ty(ty: &scalar::Ty, accept_str_eq: bool) -> TokenStream {
    use scalar::Ty;
    match ty {
        Ty::Int32
        | Ty::Int64
        | Ty::Uint32
        | Ty::Uint64
        | Ty::Sint32
        | Ty::Sint64
        | Ty::Fixed32
        | Ty::Fixed64
        | Ty::Sfixed32
        | Ty::Sfixed64 => quote! { _private::IntDeserializer },
        Ty::Float | Ty::Double => quote! { _private::FloatDeserializer },
        Ty::Bool => {
            quote! { _private::BoolDeserializer<{ #accept_str_eq }> }
        }
        Ty::String => {
            quote! { _private::ForwardDeserializer }
        }
        Ty::Enumeration(path) => {
            quote! { _private::EnumDeserializer::<#path> }
        }
        Ty::Bytes(_) => quote! { _private::BytesDeserializer },
    }
}
