include!(concat!(env!("OUT_DIR"), "/proto3_presence.rs"));

#[test]
fn test_proto3_presence() {
    let msg = A {
        b: Some(42),
        foo: Some(a::Foo::C(13)),
    };

    crate::check_message(&msg);
}
