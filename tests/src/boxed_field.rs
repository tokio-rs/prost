include!(concat!(env!("OUT_DIR"), "/boxed_field.rs"));

use self::foo::OneofField;

#[test]
/// - Confirm `Foo::bar` and `OneofField::BoxQux` is boxed by creating an instance
/// - `Foo::boxed_bar_list` should not be boxed as it is a `Vec`, therefore it is already heap allocated
fn test_boxed_field() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    let foo = Foo {
        bar: Some(Box::new(Bar {})),
        oneof_field: Some(OneofField::BoxQux(Box::new(Bar {}))),
        boxed_bar_list: Vec::from([Bar {}]),
    };
    let _ = Foo {
        oneof_field: Some(OneofField::Baz("hello".into())),
        ..foo
    };
}
