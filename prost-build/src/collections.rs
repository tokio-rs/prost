/// The map collection type to output for Protobuf `map` fields.
#[non_exhaustive]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub(crate) enum MapType {
    // IMPROVEMENT: place behind std feature flag
    /// The [`std::collections::HashMap`] type.
    #[default]
    HashMap,
    /// The [`alloc::collections::BTreeMap`] type.
    BTreeMap,
}

/// The bytes collection type to output for Protobuf `bytes` fields.
#[non_exhaustive]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub(crate) enum BytesType {
    /// The [`alloc::collections::Vec::<u8>`] type.
    #[default]
    Vec,
    /// The [`bytes::Bytes`] type.
    Bytes,
}

impl MapType {
    /// The `prost-derive` annotation type corresponding to the map type.
    pub fn annotation(&self) -> &'static str {
        match self {
            MapType::HashMap => "map",
            MapType::BTreeMap => "btree_map",
        }
    }

    /// The fully-qualified Rust type corresponding to the map type.
    pub fn rust_type(&self) -> &'static str {
        match self {
            MapType::HashMap => "HashMap",
            MapType::BTreeMap => "BTreeMap",
        }
    }
}

impl BytesType {
    /// The `prost-derive` annotation type corresponding to the bytes type.
    pub fn annotation(&self) -> &'static str {
        match self {
            BytesType::Vec => "vec",
            BytesType::Bytes => "bytes",
        }
    }

    /// The fully-qualified Rust type corresponding to the bytes type.
    pub fn rust_type(&self) -> &'static str {
        match self {
            BytesType::Vec => "Vec<u8>",
            BytesType::Bytes => "::prost::bytes::Bytes",
        }
    }
}
