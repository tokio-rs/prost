use std::convert::TryFrom;

use prost_types::feature_set::visibility_feature::DefaultSymbolVisibility;
use prost_types::feature_set::{
    EnumType, FieldPresence, JsonFormat, MessageEncoding, RepeatedFieldEncoding, Utf8Validation,
};
use prost_types::field_descriptor_proto::Type;
use prost_types::{Edition, FeatureSet, FieldDescriptorProto, FileDescriptorProto};

use super::syntax::Syntax;

#[derive(Clone, Copy)]
pub(super) struct FeatureValues {
    pub field_presence: FieldPresence,
    pub enum_type: EnumType,
    pub repeated_field_encoding: RepeatedFieldEncoding,
    pub utf8_validation: Utf8Validation,
    pub message_encoding: MessageEncoding,
    pub json_format: JsonFormat,
    pub default_symbol_visibility: Option<DefaultSymbolVisibility>,
}

impl FeatureValues {
    pub(super) fn from_file(file: &FileDescriptorProto) -> Self {
        let syntax: Syntax = file.syntax.as_deref().into();
        let edition = file
            .edition
            .and_then(|edition| Edition::try_from(edition).ok());

        let mut values = match edition {
            Some(Edition::Edition2024) => Self::defaults_for_edition2024(),
            Some(Edition::Edition2023) => Self::defaults_for_edition2023(),
            Some(Edition::Proto3) => Self::defaults_for_proto3(),
            Some(Edition::Proto2) | Some(Edition::Legacy) => Self::defaults_for_proto2(),
            _ => match syntax {
                Syntax::Proto3 | Syntax::Edition2023 => Self::defaults_for_proto3(),
                Syntax::Proto2 => Self::defaults_for_proto2(),
            },
        };

        if let Some(options) = file.options.as_ref() {
            if let Some(features) = options.features.as_ref() {
                values = values.with_overrides(features);
            }
        }

        values
    }

    fn defaults_for_proto2() -> Self {
        Self {
            field_presence: FieldPresence::Explicit,
            enum_type: EnumType::Closed,
            repeated_field_encoding: RepeatedFieldEncoding::Expanded,
            utf8_validation: Utf8Validation::None,
            message_encoding: MessageEncoding::LengthPrefixed,
            json_format: JsonFormat::LegacyBestEffort,
            default_symbol_visibility: None,
        }
    }

    fn defaults_for_proto3() -> Self {
        Self {
            field_presence: FieldPresence::Implicit,
            enum_type: EnumType::Open,
            repeated_field_encoding: RepeatedFieldEncoding::Packed,
            utf8_validation: Utf8Validation::Verify,
            message_encoding: MessageEncoding::LengthPrefixed,
            json_format: JsonFormat::Allow,
            default_symbol_visibility: None,
        }
    }

    fn defaults_for_edition2023() -> Self {
        Self {
            field_presence: FieldPresence::Explicit,
            enum_type: EnumType::Open,
            repeated_field_encoding: RepeatedFieldEncoding::Packed,
            utf8_validation: Utf8Validation::Verify,
            message_encoding: MessageEncoding::LengthPrefixed,
            json_format: JsonFormat::Allow,
            default_symbol_visibility: None,
        }
    }

    fn defaults_for_edition2024() -> Self {
        Self {
            default_symbol_visibility: Some(DefaultSymbolVisibility::ExportTopLevel),
            ..Self::defaults_for_edition2023()
        }
    }

    fn with_overrides(mut self, features: &FeatureSet) -> Self {
        if let Some(value) = features
            .field_presence
            .and_then(|value| FieldPresence::try_from(value).ok())
        {
            if value != FieldPresence::Unknown {
                self.field_presence = value;
            }
        }
        if let Some(value) = features
            .enum_type
            .and_then(|value| EnumType::try_from(value).ok())
        {
            if value != EnumType::Unknown {
                self.enum_type = value;
            }
        }
        if let Some(value) = features
            .repeated_field_encoding
            .and_then(|value| RepeatedFieldEncoding::try_from(value).ok())
        {
            if value != RepeatedFieldEncoding::Unknown {
                self.repeated_field_encoding = value;
            }
        }
        if let Some(value) = features
            .utf8_validation
            .and_then(|value| Utf8Validation::try_from(value).ok())
        {
            if value != Utf8Validation::Unknown {
                self.utf8_validation = value;
            }
        }
        if let Some(value) = features
            .message_encoding
            .and_then(|value| MessageEncoding::try_from(value).ok())
        {
            if value != MessageEncoding::Unknown {
                self.message_encoding = value;
            }
        }
        if let Some(value) = features
            .json_format
            .and_then(|value| JsonFormat::try_from(value).ok())
        {
            if value != JsonFormat::Unknown {
                self.json_format = value;
            }
        }
        if let Some(value) = features
            .default_symbol_visibility
            .and_then(|value| DefaultSymbolVisibility::try_from(value).ok())
        {
            if value != DefaultSymbolVisibility::Unknown {
                self.default_symbol_visibility = Some(value);
            }
        }

        self
    }

    pub(super) fn apply(self, features: Option<&FeatureSet>) -> Self {
        match features {
            Some(features) => self.with_overrides(features),
            None => self,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct FieldFeatures {
    presence: FieldPresence,
    repeated_encoding: RepeatedFieldEncoding,
}

impl FieldFeatures {
    pub(super) fn resolve(
        field: &FieldDescriptorProto,
        parent: FeatureValues,
        in_oneof: bool,
    ) -> Self {
        let mut values = parent;
        if let Some(options) = field.options.as_ref() {
            if let Some(features) = options.features.as_ref() {
                values = values.apply(Some(features));
            }
        }

        let mut presence = values.field_presence;
        if field.label() == prost_types::field_descriptor_proto::Label::Required {
            presence = FieldPresence::LegacyRequired;
        } else if in_oneof || field.proto3_optional.unwrap_or(false) {
            presence = FieldPresence::Explicit;
        }

        let mut repeated_encoding = values.repeated_field_encoding;
        if let Some(options) = field.options.as_ref() {
            if let Some(packed) = options.packed {
                repeated_encoding = if packed {
                    RepeatedFieldEncoding::Packed
                } else {
                    RepeatedFieldEncoding::Expanded
                };
            } else if matches!(repeated_encoding, RepeatedFieldEncoding::Packed)
                && options
                    .features
                    .as_ref()
                    .and_then(|f| f.repeated_field_encoding)
                    .is_none()
            {
                repeated_encoding = RepeatedFieldEncoding::Expanded;
            }
        }

        Self {
            presence,
            repeated_encoding,
        }
    }

    pub(super) fn is_required(&self, field: &FieldDescriptorProto) -> bool {
        field.label() == prost_types::field_descriptor_proto::Label::Required
            || self.presence == FieldPresence::LegacyRequired
    }

    pub(super) fn is_optional(&self, field: &FieldDescriptorProto) -> bool {
        if field.label() == prost_types::field_descriptor_proto::Label::Repeated {
            return false;
        }

        match field.r#type() {
            Type::Message | Type::Group => {
                field.label() != prost_types::field_descriptor_proto::Label::Required
            }
            _ => {
                if self.is_required(field) {
                    return false;
                }

                self.presence == FieldPresence::Explicit
            }
        }
    }

    pub(super) fn is_packed(&self, field: &FieldDescriptorProto) -> bool {
        if field.label() != prost_types::field_descriptor_proto::Label::Repeated {
            return false;
        }

        matches!(self.repeated_encoding, RepeatedFieldEncoding::Packed)
    }
}
