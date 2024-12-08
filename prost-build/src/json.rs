use std::iter;

use heck::{ToShoutySnakeCase, ToSnakeCase};
use itertools::Itertools;

use crate::code_generator::{EnumVariantMapping, Field};

pub fn json_attr_for_field(field: &Field) -> Option<String> {
    let rust_field_name = &field.rust_name();
    let proto_field_name = field.descriptor.name();
    let inferred_json_field_name = proto_field_name.to_proto_camel_case();

    if let Some(json_name) = field.descriptor.json_name.as_deref() {
        if json_name != inferred_json_field_name {
            return Some(format!(
                "json(proto_name = \"{}\", json_name = \"{}\")",
                proto_field_name, json_name
            ));
        }
    }

    let field_name_is_stable_for_json = rust_field_name == proto_field_name;
    if field_name_is_stable_for_json {
        // We skip emitting the `json` attribute for this case because this is inferred by the
        // derive macro.
        None
    } else {
        Some(format!("json(proto_name = \"{proto_field_name}\")"))
    }
}

pub fn json_attr_for_one_of_variant(field: &Field) -> Option<String> {
    let rust_variant_name = &field.rust_variant_name();
    let proto_field_name = field.descriptor.name();
    let inferred_json_field_name = proto_field_name.to_proto_camel_case();

    if let Some(json_name) = field.descriptor.json_name.as_deref() {
        if json_name != inferred_json_field_name {
            return Some(format!(
                "json(proto_name = \"{}\", json_name = \"{}\")",
                proto_field_name, json_name
            ));
        }
    }

    let variant_name_is_stable_for_json = rust_variant_name.to_snake_case() == proto_field_name;
    if variant_name_is_stable_for_json {
        // We skip emitting the `json` attribute for this case because this is inferred by the
        // derive macro.
        None
    } else {
        Some(format!("json(proto_name = \"{proto_field_name}\")"))
    }
}

pub fn json_attr_for_enum_variant(
    rust_enum_name: &str,
    variant: &EnumVariantMapping<'_>,
) -> Option<String> {
    let variant_name_is_stable_for_json = {
        let rust_enum_variant_name =
            format!("{}_{}", rust_enum_name, variant.generated_variant_name).to_shouty_snake_case();
        rust_enum_variant_name == variant.proto_name
    };

    let emit_proto_names = !variant.proto_aliases.is_empty() || !variant_name_is_stable_for_json;

    if emit_proto_names {
        let names = iter::once(variant.proto_name)
            .chain(variant.proto_aliases.iter().copied())
            .map(|proto_name| format!("proto_name = \"{proto_name}\""))
            .join(", ");

        Some(format!("json({names})"))
    } else {
        None
    }
}

pub trait ToProtoCamelCase: ToOwned {
    fn to_proto_camel_case(&self) -> Self::Owned;
}

impl ToProtoCamelCase for str {
    fn to_proto_camel_case(&self) -> Self::Owned {
        // Reference: https://protobuf.com/docs/language-spec#default-json-names
        //
        // If no json_name pseudo-option is present, the JSON name of the field will be
        // the field's name converted to camelCase. To convert to camelCase:
        //
        // - Discard any trailing underscores (_)
        // - When a leading or interior underscore is encountered, discard the underscore and
        //   capitalize the next non-underscore character encountered.
        // - Any other non-underscore and non-capitalized character is retained as is.
        //
        let mut capitalize_next = false;
        let mut out = String::with_capacity(self.len());
        for chr in self.chars() {
            if chr == '_' {
                capitalize_next = true;
            } else if capitalize_next {
                out.push(chr.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                out.push(chr);
            }
        }
        out
    }
}

impl ToProtoCamelCase for String {
    fn to_proto_camel_case(&self) -> Self::Owned {
        self.as_str().to_proto_camel_case()
    }
}
