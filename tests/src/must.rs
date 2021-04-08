use prost::Message;

include!(concat!(env!("OUT_DIR"), "/must.rs"));

use crate::check_serialize_equivalent;

#[test]
fn test_missing_must_errors_during_decode() {
    let uuid = uuid::Uuid::new_v4();

    let no_opts = RequestNoOpts {
        id: uuid.to_string(),
        meta: None,
    };

    let mut no_opts_buf = Vec::new();

    no_opts.encode(&mut no_opts_buf).unwrap();
    Request::decode(&no_opts_buf[..]).unwrap_err();
}

#[test]
fn test_option_works_with_required() {
    let uuid = uuid::Uuid::new_v4();

    let no_opts = RequestNoOpts {
        id: uuid.to_string(),
        meta: Some(Meta::default()),
    };

    let mut no_opts_buf = Vec::new();

    no_opts.encode(&mut no_opts_buf).unwrap();
    Request::decode(&no_opts_buf[..]).unwrap();
}

#[test]
fn test_oneof_required_serializes_equal() {
    let required = OneofRequired {
        foobar: oneof_required::Foobar::Foo(String::from("la")),
    };

    let optional = OneofOptional {
        foobar: Some(oneof_optional::Foobar::Foo(String::from("la"))),
    };

    check_serialize_equivalent(&required, &optional);
}

#[test]
fn missing_required_oneof_is_decode_error() {
    let optional = OneofOptional {
        foobar: Some(oneof_optional::Foobar::Foo(String::from("la"))),
    };

    let mut buf = Vec::new();

    optional.encode(&mut buf).unwrap();

    OneofRequired::decode(&buf[..]).unwrap();
}
