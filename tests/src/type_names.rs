use prost::Name;

include!(concat!(env!("OUT_DIR"), "/type_names.rs"));

#[test]
fn valid_type_names() {
    assert_eq!("Foo", Foo::NAME);
    assert_eq!("type_names", Foo::PACKAGE);
    assert_eq!("tests/type_names.Foo", Foo::type_url());

    assert_eq!("Bar", Bar::NAME);
    assert_eq!("type_names", Bar::PACKAGE);
    assert_eq!("/type_names.Bar", Bar::type_url());
}
