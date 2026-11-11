//! Tests for the `debug_redact` field attribute, which hides a field's value from `Debug` output.

include!(concat!(env!("OUT_DIR"), "/debug_redact.rs"));

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

#[derive(Clone, PartialEq, prost::Message)]
struct Inner {
    #[prost(string, tag = "1")]
    secret: String,
}

#[derive(Clone, PartialEq, prost::Enumeration)]
enum Level {
    Low = 0,
    High = 1,
}

#[derive(Clone, PartialEq, prost::Oneof)]
enum Choice {
    #[prost(string, tag = "10")]
    Plain(String),
    #[prost(string, tag = "11", debug_redact)]
    Hidden(String),
}

#[derive(Clone, PartialEq, prost::Message)]
struct EveryKind {
    #[prost(string, tag = "1")]
    visible: String,
    #[prost(string, tag = "2", debug_redact)]
    scalar: String,
    #[prost(int32, optional, tag = "3", debug_redact)]
    optional: Option<i32>,
    #[prost(int32, repeated, tag = "4", debug_redact)]
    repeated: Vec<i32>,
    #[prost(btree_map = "string, string", tag = "5", debug_redact)]
    map: BTreeMap<String, String>,
    #[prost(message, optional, tag = "6", debug_redact)]
    message: Option<Inner>,
    #[prost(enumeration = "Level", tag = "7", debug_redact)]
    enumeration: i32,
    #[prost(bytes = "vec", tag = "8", debug_redact)]
    bytes: Vec<u8>,
    #[prost(oneof = "Choice", tags = "10, 11", debug_redact)]
    choice: Option<Choice>,
}

/// Each kind of field prints `[REDACTED]` in place of its value.
#[test]
fn every_kind_of_field() {
    let mut map = BTreeMap::new();
    map.insert("k".to_string(), "v".to_string());
    let msg = EveryKind {
        visible: "shown".to_string(),
        scalar: "hidden".to_string(),
        optional: Some(1),
        repeated: vec![1, 2, 3],
        map,
        message: Some(Inner {
            secret: "hidden".to_string(),
        }),
        enumeration: Level::High as i32,
        bytes: b"hidden".to_vec(),
        choice: Some(Choice::Plain("hidden".to_string())),
    };
    assert_eq!(
        format!("{msg:?}"),
        "EveryKind { \
         visible: \"shown\", \
         scalar: [REDACTED], \
         optional: [REDACTED], \
         repeated: [REDACTED], \
         map: [REDACTED], \
         message: [REDACTED], \
         enumeration: [REDACTED], \
         bytes: [REDACTED], \
         choice: [REDACTED] \
         }"
    );
}

/// Empty and absent values are redacted too, so the output doesn't reveal whether a value is set.
#[test]
fn empty_values() {
    let msg = EveryKind::default();
    assert_eq!(
        format!("{msg:?}"),
        "EveryKind { \
         visible: \"\", \
         scalar: [REDACTED], \
         optional: [REDACTED], \
         repeated: [REDACTED], \
         map: [REDACTED], \
         message: [REDACTED], \
         enumeration: [REDACTED], \
         bytes: [REDACTED], \
         choice: [REDACTED] \
         }"
    );
}

/// A redacted oneof variant keeps its name, and other variants are unaffected.
#[test]
fn oneof_variants() {
    assert_eq!(
        format!("{:?}", Choice::Hidden("hidden".to_string())),
        "Hidden([REDACTED])"
    );
    assert_eq!(
        format!("{:?}", Choice::Plain("shown".to_string())),
        "Plain(\"shown\")"
    );
}

/// A tuple struct prints `[REDACTED]` in the redacted field's position.
#[test]
fn tuple_struct() {
    #[derive(Clone, PartialEq, prost::Message)]
    struct NewType(
        #[prost(string, tag = "1", debug_redact)] String,
        #[prost(int32, tag = "2")] i32,
    );
    assert_eq!(
        format!("{:?}", NewType("hidden".to_string(), 7)),
        "NewType([REDACTED], 7)"
    );
}

/// The attribute accepts an explicit boolean, in a separate `#[prost(...)]` attribute or not.
#[test]
fn explicit_values() {
    #[derive(Clone, PartialEq, prost::Message)]
    struct Explicit {
        #[prost(string, tag = "1", debug_redact = true)]
        on: String,
        #[prost(string, tag = "2", debug_redact = false)]
        off: String,
        #[prost(string, tag = "3")]
        #[prost(debug_redact = "true")]
        separate: String,
    }
    let msg = Explicit {
        on: "hidden".to_string(),
        off: "shown".to_string(),
        separate: "hidden".to_string(),
    };
    assert_eq!(
        format!("{msg:?}"),
        "Explicit { on: [REDACTED], off: \"shown\", separate: [REDACTED] }"
    );
}

/// Redaction only affects `Debug`, so redacted fields still encode and decode.
#[test]
fn encoding_is_unaffected() {
    use prost::Message;

    let msg = Login {
        user: "jeff".to_string(),
        password: "hunter2".to_string(),
        credential: Some(login::Credential::Token("abc".to_string())),
    };
    let decoded = Login::decode(msg.encode_to_vec().as_slice()).unwrap();
    assert_eq!(decoded, msg);
}

/// prost-build can add the attribute with `Config::field_attribute`.
#[test]
fn generated_by_prost_build() {
    let msg = Login {
        user: "jeff".to_string(),
        password: "hunter2".to_string(),
        credential: Some(login::Credential::Token("abc".to_string())),
    };
    assert_eq!(
        format!("{msg:?}"),
        "Login { user: \"jeff\", password: [REDACTED], credential: Some(Token([REDACTED])) }"
    );

    let msg = Login {
        credential: Some(login::Credential::Pin(1234)),
        ..msg
    };
    assert_eq!(
        format!("{msg:?}"),
        "Login { user: \"jeff\", password: [REDACTED], credential: Some(Pin(1234)) }"
    );
}
