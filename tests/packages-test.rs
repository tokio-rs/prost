#[macro_use]
extern crate proto_derive;
extern crate proto;
mod packages;

#[test]
fn test() {
    let mut factory = packages::widget::factory::WidgetFactory::default();
    factory.foo = 13;
    panic!("hello world");
}


