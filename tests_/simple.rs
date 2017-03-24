#[macro_use]
extern crate proto_derive;

extern crate proto;

#[derive(Debug, Message)]
struct MyMessage {
    #[proto(tag="1", default="13")]
    a: i32,

    #[proto(tag="2", default="13")]
    b: i32,

    #[proto(tag="3", signed_key)]
    foo: ::std::collections::HashMap<i32, String>,
}

#[derive(Clone, Debug, PartialEq, Message)]
pub struct M {
    #[proto(tag="1")]
    pub id: i32,
    /// Comment on inner.
    #[proto(tag="2")]
    pub inner: Option<m::Inner>,
    // Free comment.

    /// Comment on another_id.
    #[proto(tag="3", fixed)]
    pub another_id: i32,
    /// test_map comment
    #[proto(tag="4", fixed_value)]
    pub test_map: ::std::collections::HashMap<i32, i32>,
    /// test oneof 1 doc
    #[proto(tag="9", tag="6")]
    test_oneof_1: Option<m::TestOneof1>,
    #[proto(tag="7", tag="8", tag="5")]
    test_oneof_2: Option<m::TestOneof2>,
}
pub mod m {
    #[derive(Clone, Debug, PartialEq, Message)]
    pub struct Inner {
        #[proto(tag="1")]
        pub name: String,
        /// inner_test_map comment
        #[proto(tag="2")]
        pub inner_test_map: ::std::collections::HashMap<i32, String>,
    }
    /// test oneof 1 doc
    #[derive(Clone, Debug, PartialEq, Oneof)]
    pub enum TestOneof1 {
        /// name 1 doc
        #[proto(tag="9")]
        Name1(String),
        /// name 2 doc
        /// with another line
        #[proto(tag="6")]
        Id1(i32),
    }
    #[derive(Clone, Debug, PartialEq, Oneof)]
    pub enum TestOneof2 {
        #[proto(tag="7")]
        Name2(String),
        #[proto(tag="8")]
        Id2(i32),
        #[proto(tag="5")]
        Foo(Vec<u8>),
    }
}
