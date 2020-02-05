include!(concat!(env!("OUT_DIR"), "/well_known_types.rs"));

#[test]
fn test_well_known_types() {
    let msg = Foo {
        null: ::prost_types::NullValue::NullValue.into(),
        timestamp: Some(::prost_types::Timestamp {
            seconds: 99,
            nanos: 42,
        }),
    };

    crate::check_message(&msg);
}

use prost_types::Value;
use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};

#[test]
fn test_well_known_types_value() {
    let number: Value = Value::from(10.0);
    println!("Number: {:?}", number);
    let null: Value = Value::null();
    println!("Null: {:?}", null);
    let string: Value = Value::from(String::from("Hello"));
    println!("String: {:?}", string);
    let list = vec![Value::null(), Value::from(100.0)];
    let pb_list: Value = Value::from(list);
    println!("List: {:?}", pb_list);
    let mut map: BTreeMap<String, Value> = BTreeMap::new();
    map.insert(String::from("number"), number);
    map.insert(String::from("null"), null);
    map.insert(String::from("string"), string);
    map.insert(String::from("list"), pb_list);
    let pb_struct: Value = Value::from(map);
    println!("Struct: {:?}", pb_struct);
}

#[test]
fn test_well_known_types_convert_number() {
    let number: Value = Value::from(10.0);
    let back: f64 = number.try_into().unwrap();
    println!("{:?}", back);
    assert_eq!(10.0, back)
}

#[test]
fn test_well_known_types_convert_string() {
    let string: Value = Value::from(String::from("Hello world!"));
    let back: String = string.try_into().unwrap();
    println!("{:?}", back);
    assert_eq!("Hello world!", back)
}

#[test]
fn test_well_known_types_fail() {
    let string: Value = Value::from(String::from("Hello"));
    let back = f64::try_from(string);
    println!("{:?}", back.expect_err("Expected conversion error!"))
}