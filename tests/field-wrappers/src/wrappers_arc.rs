#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Payload {
    #[prost(int32, repeated, packed = "false", tag = "1")]
    pub stuff: ::prost::alloc::vec::Vec<i32>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MyMessage {
    #[prost(int32, required, arc, tag = "20")]
    pub int: ::prost::alloc::sync::Arc<i32>,
    #[prost(int32, optional, arc, tag = "21")]
    pub optional_int: ::prost::alloc::sync::Arc<::core::option::Option<i32>>,
    #[prost(int32, repeated, packed = "false", arc, tag = "22")]
    pub repeated_int: ::prost::alloc::sync::Arc<::prost::alloc::vec::Vec<i32>>,
    #[prost(int32, repeated, arc, tag = "23")]
    pub packed_int: ::prost::alloc::sync::Arc<::prost::alloc::vec::Vec<i32>>,
    #[prost(string, required, arc, tag = "1")]
    pub str: ::prost::alloc::sync::Arc<::prost::alloc::string::String>,
    #[prost(string, optional, arc, tag = "2")]
    pub optional_str: ::prost::alloc::sync::Arc<
        ::core::option::Option<::prost::alloc::string::String>,
    >,
    #[prost(string, repeated, arc, tag = "16")]
    pub repeated_str: ::prost::alloc::sync::Arc<
        ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    >,
    #[prost(message, required, arc, tag = "3")]
    pub payload: ::prost::alloc::sync::Arc<Payload>,
    #[prost(message, optional, arc, tag = "4")]
    pub optional_payload: ::prost::alloc::sync::Arc<::core::option::Option<Payload>>,
    #[prost(message, repeated, arc, tag = "17")]
    pub repeated_payload: ::prost::alloc::sync::Arc<::prost::alloc::vec::Vec<Payload>>,
    #[prost(btree_map = "int32, message", arc, tag = "5")]
    pub map_payload: ::prost::alloc::sync::Arc<
        ::prost::alloc::collections::BTreeMap<i32, Payload>,
    >,
    #[prost(group, required, arc, tag = "6")]
    pub group: ::prost::alloc::sync::Arc<my_message::Group>,
    #[prost(group, optional, arc, tag = "8")]
    pub optional_group: ::prost::alloc::sync::Arc<
        ::core::option::Option<my_message::OptionalGroup>,
    >,
    #[prost(group, repeated, arc, tag = "18")]
    pub repeated_group: ::prost::alloc::sync::Arc<
        ::prost::alloc::vec::Vec<my_message::RepeatedGroup>,
    >,
    #[prost(enumeration = "MyEnum", required, arc, tag = "12")]
    pub my_enum: ::prost::alloc::sync::Arc<i32>,
    #[prost(enumeration = "MyEnum", optional, arc, tag = "13")]
    pub optional_my_enum: ::prost::alloc::sync::Arc<::core::option::Option<i32>>,
    #[prost(enumeration = "MyEnum", repeated, packed = "false", arc, tag = "14")]
    pub repeated_my_enum: ::prost::alloc::sync::Arc<::prost::alloc::vec::Vec<i32>>,
    #[prost(enumeration = "MyEnum", repeated, arc, tag = "15")]
    pub packed_my_enum: ::prost::alloc::sync::Arc<::prost::alloc::vec::Vec<i32>>,
    /// default tests:
    #[prost(int32, optional, arc, tag = "24", default = "42")]
    pub default_int: ::prost::alloc::sync::Arc<::core::option::Option<i32>>,
    #[prost(float, optional, arc, tag = "25", default = "1")]
    pub default_float: ::prost::alloc::sync::Arc<::core::option::Option<f32>>,
    #[prost(string, optional, arc, tag = "26", default = "foobar")]
    pub default_string: ::prost::alloc::sync::Arc<
        ::core::option::Option<::prost::alloc::string::String>,
    >,
    #[prost(oneof = "my_message::OneofField", arc, tags = "10, 11")]
    pub oneof_field: ::prost::alloc::sync::Arc<
        ::core::option::Option<my_message::OneofField>,
    >,
}
/// Nested message and enum types in `MyMessage`.
pub mod my_message {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct Group {
        #[prost(int32, optional, tag = "7")]
        pub i2: ::core::option::Option<i32>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct OptionalGroup {
        #[prost(int32, optional, tag = "9")]
        pub i2: ::core::option::Option<i32>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct RepeatedGroup {
        #[prost(int32, optional, tag = "19")]
        pub i2: ::core::option::Option<i32>,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum OneofField {
        #[prost(string, tag = "10")]
        A(::prost::alloc::string::String),
        #[prost(bytes, tag = "11")]
        B(::prost::alloc::vec::Vec<u8>),
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum MyEnum {
    Bar = 1,
    Foo = 2,
    Baz = 3,
}
impl MyEnum {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            MyEnum::Bar => "Bar",
            MyEnum::Foo => "Foo",
            MyEnum::Baz => "Baz",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "Bar" => Some(Self::Bar),
            "Foo" => Some(Self::Foo),
            "Baz" => Some(Self::Baz),
            _ => None,
        }
    }
}
