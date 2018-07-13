//! Unit tests that ensure that the Default trait can be used with the generated
//! types.

pub mod default_trait {
	include!(concat!(env!("OUT_DIR"), "/default_trait.rs"));
}

#[test]
fn proto_message() {
	let _msg : default_trait::AMessage = Default::default();
	assert_eq!(_msg.number, 0);
	assert_eq!(_msg.field, None);
}

#[test]
fn proto_enum() {
	let _enum : default_trait::AnEnum = Default::default();
	assert_eq!(_enum, default_trait::AnEnum::A);
}
