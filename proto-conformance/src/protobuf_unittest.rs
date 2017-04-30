/// This proto includes every type of field in both singular and repeated
/// forms.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct TestAllTypes {
    /// Singular
    #[proto(tag="1")]
    pub single_int32: i32,
    #[proto(tag="2")]
    pub single_int64: i64,
    #[proto(tag="3")]
    pub single_uint32: u32,
    #[proto(tag="4")]
    pub single_uint64: u64,
    #[proto(tag="5", signed)]
    pub single_sint32: i32,
    #[proto(tag="6", signed)]
    pub single_sint64: i64,
    #[proto(tag="7", fixed)]
    pub single_fixed32: u32,
    #[proto(tag="8", fixed)]
    pub single_fixed64: u64,
    #[proto(tag="9", fixed)]
    pub single_sfixed32: i32,
    #[proto(tag="10", fixed)]
    pub single_sfixed64: i64,
    #[proto(tag="11")]
    pub single_float: f32,
    #[proto(tag="12")]
    pub single_double: f64,
    #[proto(tag="13")]
    pub single_bool: bool,
    #[proto(tag="14")]
    pub single_string: String,
    #[proto(tag="15")]
    pub single_bytes: Vec<u8>,
    #[proto(tag="18")]
    pub single_nested_message: Option<test_all_types::NestedMessage>,
    #[proto(tag="19")]
    pub single_foreign_message: Option<ForeignMessage>,
    #[proto(tag="20")]
    pub single_import_message: Option<super::protobuf_unittest_import::ImportMessage>,
    #[proto(tag="21", enumeration)]
    pub single_nested_enum: test_all_types::NestedEnum,
    #[proto(tag="22", enumeration)]
    pub single_foreign_enum: ForeignEnum,
    #[proto(tag="23", enumeration)]
    pub single_import_enum: super::protobuf_unittest_import::ImportEnum,
    /// Defined in unittest_import_public.proto
    #[proto(tag="26")]
    pub single_public_import_message: Option<super::protobuf_unittest_import::PublicImportMessage>,
    /// Repeated
    #[proto(tag="31")]
    pub repeated_int32: Vec<i32>,
    #[proto(tag="32")]
    pub repeated_int64: Vec<i64>,
    #[proto(tag="33")]
    pub repeated_uint32: Vec<u32>,
    #[proto(tag="34")]
    pub repeated_uint64: Vec<u64>,
    #[proto(tag="35", signed)]
    pub repeated_sint32: Vec<i32>,
    #[proto(tag="36", signed)]
    pub repeated_sint64: Vec<i64>,
    #[proto(tag="37", fixed)]
    pub repeated_fixed32: Vec<u32>,
    #[proto(tag="38", fixed)]
    pub repeated_fixed64: Vec<u64>,
    #[proto(tag="39", fixed)]
    pub repeated_sfixed32: Vec<i32>,
    #[proto(tag="40", fixed)]
    pub repeated_sfixed64: Vec<i64>,
    #[proto(tag="41")]
    pub repeated_float: Vec<f32>,
    #[proto(tag="42")]
    pub repeated_double: Vec<f64>,
    #[proto(tag="43")]
    pub repeated_bool: Vec<bool>,
    #[proto(tag="44")]
    pub repeated_string: Vec<String>,
    #[proto(tag="45")]
    pub repeated_bytes: Vec<Vec<u8>>,
    #[proto(tag="48")]
    pub repeated_nested_message: Vec<test_all_types::NestedMessage>,
    #[proto(tag="49")]
    pub repeated_foreign_message: Vec<ForeignMessage>,
    #[proto(tag="50")]
    pub repeated_import_message: Vec<super::protobuf_unittest_import::ImportMessage>,
    #[proto(tag="51", enumeration)]
    pub repeated_nested_enum: Vec<test_all_types::NestedEnum>,
    #[proto(tag="52", enumeration)]
    pub repeated_foreign_enum: Vec<ForeignEnum>,
    #[proto(tag="53", enumeration)]
    pub repeated_import_enum: Vec<super::protobuf_unittest_import::ImportEnum>,
    /// Defined in unittest_import_public.proto
    #[proto(tag="54")]
    pub repeated_public_import_message: Vec<super::protobuf_unittest_import::PublicImportMessage>,
    /// For oneof test
    #[proto(tag="111", tag="112", tag="113", tag="114", oneof)]
    oneof_field: Option<test_all_types::OneofField>,
}
pub mod test_all_types {
    #[derive(Clone, Debug, PartialEq, Message)]
    pub struct NestedMessage {
        /// The field name "b" fails to compile in proto1 because it conflicts with
        /// a local variable named "b" in one of the generated methods.  Doh.
        /// This file needs to compile in proto1 to test backwards-compatibility.
        #[proto(tag="1")]
        pub bb: i32,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Enumeration)]
    pub enum NestedEnum {
        NestedEnumUnspecified = 0,
        Foo = 1,
        Bar = 2,
        Baz = 3,
        /// Intentionally negative.
        Neg = -1,
    }
    /// For oneof test
    #[derive(Clone, Debug, PartialEq, Oneof)]
    pub enum OneofField {
        #[proto(tag="111")]
        OneofUint32(u32),
        #[proto(tag="112")]
        OneofNestedMessage(NestedMessage),
        #[proto(tag="113")]
        OneofString(String),
        #[proto(tag="114")]
        OneofBytes(Vec<u8>),
    }
}
/// This proto includes a recusively nested message.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct NestedTestAllTypes {
    #[proto(tag="1")]
    pub child: Option<NestedTestAllTypes>,
    #[proto(tag="2")]
    pub payload: Option<TestAllTypes>,
    #[proto(tag="3")]
    pub repeated_child: Vec<NestedTestAllTypes>,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct TestDeprecatedFields {
    #[proto(tag="1")]
    pub deprecated_int32: i32,
}
/// Define these after TestAllTypes to make sure the compiler can handle
/// that.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct ForeignMessage {
    #[proto(tag="1")]
    pub c: i32,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct TestReservedFields {
}
/// Test that we can use NestedMessage from outside TestAllTypes.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct TestForeignNested {
    #[proto(tag="1")]
    pub foreign_nested: Option<test_all_types::NestedMessage>,
}
/// Test that really large tag numbers don't break anything.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct TestReallyLargeTagNumber {
    /// The largest possible tag number is 2^28 - 1, since the wire format uses
    /// three bits to communicate wire type.
    #[proto(tag="1")]
    pub a: i32,
    #[proto(tag="268435455")]
    pub bb: i32,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct TestRecursiveMessage {
    #[proto(tag="1")]
    pub a: Option<TestRecursiveMessage>,
    #[proto(tag="2")]
    pub i: i32,
}
/// Test that mutual recursion works.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct TestMutualRecursionA {
    #[proto(tag="1")]
    pub bb: Option<TestMutualRecursionB>,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct TestMutualRecursionB {
    #[proto(tag="1")]
    pub a: Option<TestMutualRecursionA>,
    #[proto(tag="2")]
    pub optional_int32: i32,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct TestEnumAllowAlias {
    #[proto(tag="1", enumeration)]
    pub value: TestEnumWithDupValue,
}
/// Test message with CamelCase field names.  This violates Protocol Buffer
/// standard style.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct TestCamelCaseFieldNames {
    #[proto(tag="1")]
    pub PrimitiveField: i32,
    #[proto(tag="2")]
    pub StringField: String,
    #[proto(tag="3", enumeration)]
    pub EnumField: ForeignEnum,
    #[proto(tag="4")]
    pub MessageField: Option<ForeignMessage>,
    #[proto(tag="7")]
    pub RepeatedPrimitiveField: Vec<i32>,
    #[proto(tag="8")]
    pub RepeatedStringField: Vec<String>,
    #[proto(tag="9", enumeration)]
    pub RepeatedEnumField: Vec<ForeignEnum>,
    #[proto(tag="10")]
    pub RepeatedMessageField: Vec<ForeignMessage>,
}
/// We list fields out of order, to ensure that we're using field number and not
/// field index to determine serialization order.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct TestFieldOrderings {
    #[proto(tag="11")]
    pub my_string: String,
    #[proto(tag="1")]
    pub my_int: i64,
    #[proto(tag="101")]
    pub my_float: f32,
    #[proto(tag="200")]
    pub single_nested_message: Option<test_field_orderings::NestedMessage>,
}
pub mod test_field_orderings {
    #[derive(Clone, Debug, PartialEq, Message)]
    pub struct NestedMessage {
        #[proto(tag="2")]
        pub oo: i64,
        /// The field name "b" fails to compile in proto1 because it conflicts with
        /// a local variable named "b" in one of the generated methods.  Doh.
        /// This file needs to compile in proto1 to test backwards-compatibility.
        #[proto(tag="1")]
        pub bb: i32,
    }
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct SparseEnumMessage {
    #[proto(tag="1", enumeration)]
    pub sparse_enum: TestSparseEnum,
}
/// Test String and Bytes: string is for valid UTF-8 strings
#[derive(Clone, Debug, PartialEq, Message)]
pub struct OneString {
    #[proto(tag="1")]
    pub data: String,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct MoreString {
    #[proto(tag="1")]
    pub data: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct OneBytes {
    #[proto(tag="1")]
    pub data: Vec<u8>,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct MoreBytes {
    #[proto(tag="1")]
    pub data: Vec<u8>,
}
/// Test int32, uint32, int64, uint64, and bool are all compatible
#[derive(Clone, Debug, PartialEq, Message)]
pub struct Int32Message {
    #[proto(tag="1")]
    pub data: i32,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct Uint32Message {
    #[proto(tag="1")]
    pub data: u32,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct Int64Message {
    #[proto(tag="1")]
    pub data: i64,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct Uint64Message {
    #[proto(tag="1")]
    pub data: u64,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct BoolMessage {
    #[proto(tag="1")]
    pub data: bool,
}
/// Test oneofs.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct TestOneof {
    #[proto(tag="1", tag="2", tag="3", oneof)]
    foo: Option<test_oneof::Foo>,
}
pub mod test_oneof {
    #[derive(Clone, Debug, PartialEq, Oneof)]
    pub enum Foo {
        #[proto(tag="1")]
        FooInt(i32),
        #[proto(tag="2")]
        FooString(String),
        #[proto(tag="3")]
        FooMessage(super::TestAllTypes),
    }
}
// Test messages for packed fields

#[derive(Clone, Debug, PartialEq, Message)]
pub struct TestPackedTypes {
    #[proto(tag="90")]
    pub packed_int32: Vec<i32>,
    #[proto(tag="91")]
    pub packed_int64: Vec<i64>,
    #[proto(tag="92")]
    pub packed_uint32: Vec<u32>,
    #[proto(tag="93")]
    pub packed_uint64: Vec<u64>,
    #[proto(tag="94", signed)]
    pub packed_sint32: Vec<i32>,
    #[proto(tag="95", signed)]
    pub packed_sint64: Vec<i64>,
    #[proto(tag="96", fixed)]
    pub packed_fixed32: Vec<u32>,
    #[proto(tag="97", fixed)]
    pub packed_fixed64: Vec<u64>,
    #[proto(tag="98", fixed)]
    pub packed_sfixed32: Vec<i32>,
    #[proto(tag="99", fixed)]
    pub packed_sfixed64: Vec<i64>,
    #[proto(tag="100")]
    pub packed_float: Vec<f32>,
    #[proto(tag="101")]
    pub packed_double: Vec<f64>,
    #[proto(tag="102")]
    pub packed_bool: Vec<bool>,
    #[proto(tag="103", enumeration)]
    pub packed_enum: Vec<ForeignEnum>,
}
/// A message with the same fields as TestPackedTypes, but without packing. Used
/// to test packed <-> unpacked wire compatibility.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct TestUnpackedTypes {
    #[proto(tag="90")]
    pub unpacked_int32: Vec<i32>,
    #[proto(tag="91")]
    pub unpacked_int64: Vec<i64>,
    #[proto(tag="92")]
    pub unpacked_uint32: Vec<u32>,
    #[proto(tag="93")]
    pub unpacked_uint64: Vec<u64>,
    #[proto(tag="94", signed)]
    pub unpacked_sint32: Vec<i32>,
    #[proto(tag="95", signed)]
    pub unpacked_sint64: Vec<i64>,
    #[proto(tag="96", fixed)]
    pub unpacked_fixed32: Vec<u32>,
    #[proto(tag="97", fixed)]
    pub unpacked_fixed64: Vec<u64>,
    #[proto(tag="98", fixed)]
    pub unpacked_sfixed32: Vec<i32>,
    #[proto(tag="99", fixed)]
    pub unpacked_sfixed64: Vec<i64>,
    #[proto(tag="100")]
    pub unpacked_float: Vec<f32>,
    #[proto(tag="101")]
    pub unpacked_double: Vec<f64>,
    #[proto(tag="102")]
    pub unpacked_bool: Vec<bool>,
    #[proto(tag="103", enumeration)]
    pub unpacked_enum: Vec<ForeignEnum>,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct TestRepeatedScalarDifferentTagSizes {
    /// Parsing repeated fixed size values used to fail. This message needs to be
    /// used in order to get a tag of the right size; all of the repeated fields
    /// in TestAllTypes didn't trigger the check.
    #[proto(tag="12", fixed)]
    pub repeated_fixed32: Vec<u32>,
    /// Check for a varint type, just for good measure.
    #[proto(tag="13")]
    pub repeated_int32: Vec<i32>,
    /// These have two-byte tags.
    #[proto(tag="2046", fixed)]
    pub repeated_fixed64: Vec<u64>,
    #[proto(tag="2047")]
    pub repeated_int64: Vec<i64>,
    /// Three byte tags.
    #[proto(tag="262142")]
    pub repeated_float: Vec<f32>,
    #[proto(tag="262143")]
    pub repeated_uint64: Vec<u64>,
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct TestCommentInjectionMessage {
    /// */ <- This should not close the generated doc comment
    #[proto(tag="1")]
    pub a: String,
}
/// Test that RPC services work.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct FooRequest {
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct FooResponse {
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct FooClientMessage {
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct FooServerMessage {
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct BarRequest {
}
#[derive(Clone, Debug, PartialEq, Message)]
pub struct BarResponse {
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enumeration)]
pub enum ForeignEnum {
    ForeignUnspecified = 0,
    ForeignFoo = 4,
    ForeignBar = 5,
    ForeignBaz = 6,
}
/// Test an enum that has multiple values with the same number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enumeration)]
pub enum TestEnumWithDupValue {
    TestEnumWithDupValueUnspecified = 0,
    Foo1 = 1,
    Bar1 = 2,
    Baz = 3,
    Foo2 = 1,
    Bar2 = 2,
}
/// Test an enum with large, unordered values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Enumeration)]
pub enum TestSparseEnum {
    TestSparseEnumUnspecified = 0,
    SparseA = 123,
    SparseB = 62374,
    SparseC = 12589234,
    SparseD = -15,
    SparseE = -53452,
    /// In proto3, value 0 must be the first one specified
    /// SPARSE_F = 0;
    SparseG = 2,
}
