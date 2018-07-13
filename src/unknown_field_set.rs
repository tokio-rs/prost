//! Runtime library code for storing unknown fields.

/// A set of Protobuf fields that were not recognized during decoding.
///
/// Every Message struct should have an UnknownFieldSet member. This is how
/// messages make sure to not discard unknown data in a decode/encode cycle,
/// which is required by the Protobuf spec.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct UnknownFieldSet {
}
