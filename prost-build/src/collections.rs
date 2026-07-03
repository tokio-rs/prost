/// The map collection type to output for Protobuf `map` fields.
#[non_exhaustive]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub(crate) enum MapType {
    /// The [`std::collections::HashMap`] type.
    #[default]
    HashMap,
    /// The [`std::collections::BTreeMap`] type.
    BTreeMap,
}

impl MapType {
    /// The `prost-derive` annotation type corresponding to the map type.
    pub fn annotation(&self) -> &'static str {
        match self {
            MapType::HashMap => "map",
            MapType::BTreeMap => "btree_map",
        }
    }
}
