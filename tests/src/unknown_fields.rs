use prost::{
    EncodeError,
    Message
};

pub mod unknown_fields {
    include!(concat!(env!("OUT_DIR"), "/unknown_fields.rs"));
}

// Verify that the derive macro supports unknown_field_set properly.
#[derive(Clone, PartialEq, Message)]
pub struct EmptyMessage {
    #[prost(unknown_field_set)]
    pub unknown_fields: ::prost::UnknownFieldSet,
}

// Verify that the derive macro supports unknown_field_set properly.
#[derive(Clone, PartialEq, Message)]
pub struct NonstandardUnknownFieldSetName {
    #[prost(unknown_field_set)]
    pub intentionally_nonstandard_name: ::prost::UnknownFieldSet,
}

#[derive(Clone, PartialEq, Message)]
struct Person {
    #[prost(string, tag="1")]
    pub name: String,
    // UnknownFieldSet intentionally omitted.
}

#[derive(Clone, PartialEq, Message)]
struct PersonWithAge {
    #[prost(string, tag="1")]
    pub name: String,
    #[prost(int32, tag="2")]
    pub age: i32,
    // UnknownFieldSet intentionally omitted.
}

fn encode<T>(msg: &T) -> Result<Vec<u8>, EncodeError> where T: Message {
    let mut buf = Vec::new();
    msg.encode(&mut buf)?;
    Ok(buf)
}

#[test]
fn unknown_fields_are_kept() {
    let person_with_age = PersonWithAge { name:"Pwa".to_string(), age:12 };
    let unrelated_message = unknown_fields::MessageWithAllTypes::decode(
        encode(&person_with_age).unwrap()).unwrap();
    let reparsed_person_with_age =
        PersonWithAge::decode(encode(&unrelated_message).unwrap()).unwrap();

    assert_eq!(reparsed_person_with_age.name, "Pwa");
    assert_eq!(reparsed_person_with_age.age, 12);
    assert_eq!(
        reparsed_person_with_age.encoded_len(),
        unrelated_message.encoded_len());
}

#[test]
fn struct_without_unknown_field_set_discards_unknown_fields() {
    // It should be possible to work with structs without an UnknownFieldSet.

    let person_with_age = PersonWithAge { name:"Pwa".to_string(), age:12 };
    let person = Person::decode(encode(&person_with_age).unwrap()).unwrap();
    let reparsed_person_with_age =
        PersonWithAge::decode(encode(&person).unwrap()).unwrap();

    assert_eq!(reparsed_person_with_age.name, "Pwa");
    assert_eq!(reparsed_person_with_age.age, 0);
}

#[test]
fn unknown_fields_of_struct_with_nonstandard_set_name_are_kept() {
    // It should be possible to call the unknown_field_set something else.

    let person_with_age = PersonWithAge { name:"Pwa".to_string(), age:12 };
    let unrelated_message = NonstandardUnknownFieldSetName::decode(
        encode(&person_with_age).unwrap()).unwrap();
    let reparsed_person_with_age =
        PersonWithAge::decode(encode(&unrelated_message).unwrap()).unwrap();

    assert_eq!(reparsed_person_with_age.name, "Pwa");
    assert_eq!(reparsed_person_with_age.age, 12);
    assert_eq!(
        reparsed_person_with_age.encoded_len(),
        unrelated_message.encoded_len());
}

#[test]
fn overwrite_unknown_field_set_resets_unknown_fields() {
    let person_with_age = PersonWithAge { name:"Pwa".to_string(), age:12 };

    let mut unrelated_message = EmptyMessage::decode(
        encode(&person_with_age).unwrap()).unwrap();
    unrelated_message.unknown_fields = Default::default();

    let reparsed_person_with_age =
        PersonWithAge::decode(encode(&unrelated_message).unwrap()).unwrap();

    assert_eq!(reparsed_person_with_age.name, "");
    assert_eq!(reparsed_person_with_age.age, 0);
}

#[test]
fn unknown_varint_fields_are_kept() {
    let message = unknown_fields::MessageWithAllTypes {
        varint: 1337,
        ..Default::default()
    };
    let unrelated_message = EmptyMessage::decode(
        encode(&message).unwrap()).unwrap();
    let reparsed_message =
        unknown_fields::MessageWithAllTypes::decode(
            encode(&unrelated_message).unwrap()).unwrap();

    assert_eq!(reparsed_message.varint, 1337);
    assert_eq!(
        reparsed_message.encoded_len(),
        unrelated_message.unknown_fields.encoded_len());
}

#[test]
fn unknown_fixed32_fields_are_kept() {
    let message = unknown_fields::MessageWithAllTypes {
        thirty_two_bit: 1337,
        ..Default::default()
    };
    let unrelated_message = EmptyMessage::decode(
        encode(&message).unwrap()).unwrap();
    let reparsed_message =
        unknown_fields::MessageWithAllTypes::decode(
            encode(&unrelated_message).unwrap()).unwrap();

    assert_eq!(reparsed_message.thirty_two_bit, 1337);
    assert_eq!(
        reparsed_message.encoded_len(),
        unrelated_message.unknown_fields.encoded_len());
}

#[test]
fn unknown_fixed64_fields_are_kept() {
    let message = unknown_fields::MessageWithAllTypes {
        sixty_four_bit: 1337,
        ..Default::default()
    };
    let unrelated_message = EmptyMessage::decode(
        encode(&message).unwrap()).unwrap();
    let reparsed_message =
        unknown_fields::MessageWithAllTypes::decode(
            encode(&unrelated_message).unwrap()).unwrap();

    assert_eq!(reparsed_message.sixty_four_bit, 1337);
    assert_eq!(
        reparsed_message.encoded_len(),
        unrelated_message.unknown_fields.encoded_len());
}

#[test]
fn unknown_length_delimited_fields_are_kept() {
    let message = unknown_fields::MessageWithAllTypes {
        length_delimited: "abc".to_string(),
        ..Default::default()
    };
    let unrelated_message = EmptyMessage::decode(
        encode(&message).unwrap()).unwrap();
    let reparsed_message =
        unknown_fields::MessageWithAllTypes::decode(
            encode(&unrelated_message).unwrap()).unwrap();

    assert_eq!(reparsed_message.length_delimited, "abc");
    assert_eq!(
        reparsed_message.encoded_len(),
        unrelated_message.unknown_fields.encoded_len());
}

#[test]
fn truncated_varint_fields_fail_to_parse() {
    let message = unknown_fields::MessageWithAllTypes {
        varint: 1337,
        ..Default::default()
    };
    let mut buf = encode(&message).unwrap();
    buf.pop();

    let result = EmptyMessage::decode(buf);

    assert!(result.is_err());
}

#[test]
fn truncated_fixed32_fields_fail_to_parse() {
    let message = unknown_fields::MessageWithAllTypes {
        thirty_two_bit: 1337,
        ..Default::default()
    };
    let mut buf = encode(&message).unwrap();
    buf.pop();

    let result = EmptyMessage::decode(buf);

    assert!(result.is_err());
}

#[test]
fn truncated_fixed64_fields_fail_to_parse() {
    let message = unknown_fields::MessageWithAllTypes {
        sixty_four_bit: 1337,
        ..Default::default()
    };
    let mut buf = encode(&message).unwrap();
    buf.pop();

    let result = EmptyMessage::decode(buf);

    assert!(result.is_err());
}

#[test]
fn truncated_length_delimited_fields_fail_to_parse() {
    let message = unknown_fields::MessageWithAllTypes {
        length_delimited: "abc".to_string(),
        ..Default::default()
    };
    let mut buf = encode(&message).unwrap();
    buf.pop();

    let result = EmptyMessage::decode(buf);

    assert!(result.is_err());
}
