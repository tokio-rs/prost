include!(concat!(env!("OUT_DIR"), "/boxed_field.rs"));
use foo::OneofField;

#[test]
/// Confirm `Foo::bar` is boxed by creating an instance
fn test_bar_is_boxed() {
    use alloc::boxed::Box;
    let _ = Foo {
        bar: Some(Box::new(Bar {})),
        oneof_field: Some(OneofField::BoxQux(Box::new(Bar {}))),
    };
}
