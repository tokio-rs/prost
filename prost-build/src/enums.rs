use std::collections::HashSet;

use crate::path::fq_package_path;
use crate::syntax::Syntax;
use prost_types::{DescriptorProto, EnumDescriptorProto, FileDescriptorProto};

/// Determines how to represent a Proto enum field value in Rust.
pub enum EnumRepr {
    /// Open enumeration, represented with `OpenEnum` wrapping the Rust enum type.
    Open,
    /// Closed enumeration, represented with the Rust enum type directly.
    Closed,
    /// i32 representation.
    Int,
}

/// Registry of enum type features tracked across Protobuf files in the input.
pub struct EnumFeatures {
    // Set of fully qualified names of enums that shall be represented as closed.
    // As enums are open in proto3 (and, by default, in edition 2023 and later),
    // tracking only closed enums should make a smaller set on inputs
    // predominantly using proto3 and, in the future, editions.
    closed_enums: HashSet<String>,
}

impl EnumFeatures {
    pub(crate) fn new<'a>(files: impl Iterator<Item = &'a FileDescriptorProto>) -> Self {
        let mut enum_type_map = EnumFeatures {
            closed_enums: HashSet::new(),
        };
        for file in files {
            let syntax = Syntax::from(file.syntax.as_deref());
            // Until support for editions is added, we only need to look into
            // proto2 files to collect closed enums.
            // With edition syntax, the enum_type feature will be available to
            // override per file or individual enum.
            match syntax {
                Syntax::Proto2 => {
                    let package = fq_package_path(file);
                    enum_type_map.add_enum_types(&package, &file.enum_type);
                    for msg in &file.message_type {
                        enum_type_map.visit_message_type(&package, msg);
                    }
                }
                Syntax::Proto3 => {} // Proto3 does not have closed enums.
            }
        }
        enum_type_map
    }

    fn add_enum_types(&mut self, fq_path: &str, enum_types: &[EnumDescriptorProto]) {
        for enum_type in enum_types {
            let enum_path = format!("{}.{}", fq_path, enum_type.name());
            self.closed_enums.insert(enum_path);
        }
    }

    fn visit_message_type(&mut self, fq_path: &str, msg: &DescriptorProto) {
        let message_path = format!("{}.{}", fq_path, msg.name());
        self.add_enum_types(&message_path, &msg.enum_type);
        for msg in &msg.nested_type {
            self.visit_message_type(&message_path, msg);
        }
    }

    /// Returns true if the enum with the given fully qualified path is closed.
    pub fn is_closed(&self, fq_path: &str) -> bool {
        self.closed_enums.contains(fq_path)
    }
}
