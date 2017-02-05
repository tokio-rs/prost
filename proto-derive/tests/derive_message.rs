#[macro_use]
extern crate proto_derive;

extern crate proto;

use proto::Message;

#[derive(Debug, Default, Message)]
struct Foo {
    #[proto(tag = "42")]
    bar: f32,
}
