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
