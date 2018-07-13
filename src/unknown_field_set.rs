//! Runtime library code for storing unknown fields.

/// A set of Protobuf fields that were not recognized during decoding.
///
/// Every Message struct should have an UnknownFieldSet member. This is how
/// messages make sure to not discard unknown data in a decode/encode cycle,
/// which is required by the Protobuf spec.
#[derive(Clone, Debug, PartialEq, Eq, Default, PartialOrd, Ord)]
pub struct UnknownFieldSet {
    // The actual data of this struct is wrapped in a Box to ensure that
    // this struct uses only one machine word of memory unless there are
    // unknown fields to store.
    //
    // If the Option is non-empty, the Vec is also non-empty.
    data: Option<Box<Vec<UnknownField>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct UnknownField {
    tag: u32,
    data: UnknownFieldData,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum UnknownFieldData {
    Varint(u64),
    SixtyFourBit(u64),
    LengthDelimited(u64),
    Group(UnknownFieldSet),
    ThirtyTwoBit(u32),
}
