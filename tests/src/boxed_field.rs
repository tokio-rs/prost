include!(concat!(env!("OUT_DIR"), "/boxed_field.rs"));

#[test]
/// Confirm `Foo::bar` is boxed by creating an instance
fn test_bar_is_boxed() {
    use alloc::boxed::Box;
    let _ = Foo {
        bar: Some(Box::new(Bar {})),
    };
}
