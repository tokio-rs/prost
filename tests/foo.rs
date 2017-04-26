#[macro_use]
extern crate proto_derive;

extern crate proto;

use proto::Message;

use std::io::{
    Cursor,
};

#[derive(Debug, Message)]
struct A {
    #[proto(tag="1")]
    a: i32,

    #[proto(tag="2")]
    b: i32,
}

#[derive(Debug, Message)]
struct B {
    #[proto(tag="1")]
    a: Vec<A>,
}

/// A message representing a option the parser does not recognize. This only
/// appears in options protos created by the compiler::Parser class.
/// DescriptorPool resolves these when building Descriptor objects. Therefore,
/// options protos in descriptor objects (e.g. returned by Descriptor::options(),
/// or produced by Descriptor::CopyTo()) will never have UninterpretedOptions
/// in them.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct UninterpretedOption {
    #[proto(tag="2")]
    pub name: Vec<uninterpreted_option::NamePart>,
    /// The value of the uninterpreted option, in whatever type the tokenizer
    /// identified it as during parsing. Exactly one of these should be set.
    #[proto(tag="3")]
    pub identifier_value: String,
    #[proto(tag="4")]
    pub positive_int_value: u64,
    #[proto(tag="5")]
    pub negative_int_value: i64,
    #[proto(tag="6")]
    pub double_value: f64,
    #[proto(tag="7")]
    pub string_value: Vec<u8>,
    #[proto(tag="8")]
    pub aggregate_value: String,
}
pub mod uninterpreted_option {
    /// The name of the uninterpreted option.  Each string represents a segment in
    /// a dot-separated name.  is_extension is true iff a segment represents an
    /// extension (denoted with parentheses in options specs in .proto files).
    /// E.g.,{ ["foo", false], ["bar.baz", true], ["qux", false] } represents
    /// "foo.(bar.baz).qux".
    #[derive(Clone, Debug, PartialEq, Message)]
    pub struct NamePart {
        #[proto(tag="1")]
        pub name_part: String,
        #[proto(tag="2")]
        pub is_extension: bool,
    }
}

#[test]
fn foo() {

    UninterpretedOption::decode_length_delimited(&mut Cursor::new(vec![0u8])).unwrap();

}
