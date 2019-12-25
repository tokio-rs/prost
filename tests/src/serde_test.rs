use prost::{MessageSerde, Message};
use serde_json::json;

#[derive(Clone, PartialEq, ::prost::Message, Serialize, Deserialize)]
#[prost(package="serde.test")]
#[prost(serde)]
#[serde(default, rename_all="camelCase")]
pub struct Foo {
    #[prost(string, tag="1")]
    pub string: std::string::String,
    #[prost(message, optional, tag="2")]
    pub timestamp: ::std::option::Option<::prost_types::Timestamp>,
    #[prost(bool, tag="3")]
    pub boolean: bool,
    #[prost(message, optional, tag="4")]
    pub data: ::std::option::Option<::prost_types::Value>,
    #[prost(string, repeated, tag="5")]
    pub list: ::std::vec::Vec<std::string::String>,
    #[prost(message, optional, tag="6")]
    pub payload: ::std::option::Option<::prost_types::Any>,
}

use serde::{Deserialize, Serialize};

#[test]
fn test_well_known_types_serde_serialize_only() {
    let inner = Foo {
        string: String::from("inner"),
        timestamp: None,
        boolean: false,
        data: None,
        list: vec!["een".to_string(), "twee".to_string()],
        payload: None
    };

    let msg = Foo {
        string: String::from("hello"),
        timestamp: Some(prost_types::Timestamp::new(99, 42)),
        boolean: true,
        data: Some(prost_types::Value::from("world".to_string())),
        list: vec!["one".to_string(), "two".to_string()],
        payload: Some(prost_types::Any::pack(inner))
    };
    println!("Serialized to string: {}", serde_json::to_string(&msg).unwrap());
    let erased = &msg as &dyn MessageSerde;
    let json = serde_json::to_string(erased).unwrap();
    println!("Erased json: {}", json);
}

#[test]
fn test_well_known_types_serde_deserialize_default() {
    let type_url = "type.googleapis.com/serde.test.Foo";
    let data = json!({
        "@type": type_url,
        "value": {}
    });
    let erased: Box<dyn MessageSerde> = serde_json::from_value(data).unwrap();
    let foo: &Foo = erased.downcast_ref::<Foo>().unwrap();
    println!("Deserialize default: {:?}", foo);
}

#[test]
fn test_well_known_types_serde_deserialize_string() {
    let data =
        r#"{
            "string":"hello",
            "timestamp":"1970-01-01T00:01:39.000000042Z",
            "boolean":true,
            "data": {
              "test_number": 1,
              "test_bool": true,
              "test_string": "hi there",
              "test_list": [1, 2, 3, 4],
              "test_inner_struct": {
                "one": 1,
                "two": 2
              }
            },
            "list": []
          }"#;
    let msg: Foo = serde_json::from_str(data).unwrap();
    println!("Deserialized from string: {:?}", msg);
}

#[test]
fn test_well_known_types_serde_deserialize_any_string() {
    let data =
        r#"{
                "@type":"type.googleapis.com/serde.test.Foo",
                "string":"inner",
                "timestamp":null,
                "boolean":false,
                "data":null,
                "list":["een","twee"],
                "payload":null
           }"#;
    let any: prost_types::Any = serde_json::from_str(data).unwrap();
    println!("Deserialized any from string: {:?}", any);
    let foo: Foo = any.unpack(Foo::default()).unwrap();
    println!("Unpacked Any: {:?}", &foo);
    assert_eq!(foo.list, vec!["een", "twee"])
}

#[test]
fn test_well_known_types_serde_serialize_deserialize() {
    let inner = Foo {
        string: String::from("inner"),
        timestamp: None,
        boolean: false,
        data: None,
        list: vec!["een".to_string(), "twee".to_string()],
        payload: None
    };

    let original = Foo {
        string: String::from("original"),
        timestamp: Some(prost_types::Timestamp::new(99, 42)),
        boolean: true,
        data: Some(prost_types::Value::from("world".to_string())),
        list: vec!["one".to_string(), "two".to_string()],
        payload: Some(prost_types::Any::pack(inner))
    };

    let json = serde_json::to_string(&original).unwrap();
    println!("Serialized Foo: {}", json);
    let back: Foo = serde_json::from_str(&json).unwrap();
    println!("Deserialized Foo: {:?}", &back);
    assert_eq!(back, original)
}



