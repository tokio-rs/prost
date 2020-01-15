[![Build Status](https://travis-ci.org/danburkert/prost.svg?branch=master)](https://travis-ci.org/danburkert/prost)
[![Windows Build Status](https://ci.appveyor.com/api/projects/status/24rpba3x2vqe8lje/branch/master?svg=true)](https://ci.appveyor.com/project/danburkert/prost/branch/master)
[![Documentation](https://docs.rs/prost/badge.svg)](https://docs.rs/prost/)
[![Crate](https://img.shields.io/crates/v/prost.svg)](https://crates.io/crates/prost)

# *PROST!*

`prost` is a [Protocol Buffers](https://developers.google.com/protocol-buffers/)
implementation for the [Rust Language](https://www.rust-lang.org/). `prost`
generates simple, idiomatic Rust code from `proto2` and `proto3` files.

Compared to other Protocol Buffers implementations, `prost`

* Generates simple, idiomatic, and readable Rust types by taking advantage of
  Rust `derive` attributes.
* Retains comments from `.proto` files in generated Rust code.
* Allows existing Rust types (not generated from a `.proto`) to be serialized
  and deserialized by adding attributes.
* Uses the [`bytes::{Buf, BufMut}`](https://github.com/carllerche/bytes)
  abstractions for serialization instead of `std::io::{Read, Write}`.
* Respects the Protobuf `package` specifier when organizing generated code
  into Rust modules.
* Preserves unknown enum values during deserialization.
* Does not include support for runtime reflection or message descriptors.

## Using `prost` in a Cargo Project

First, add `prost` and its public dependencies to your `Cargo.toml`:

```
[dependencies]
prost = "0.6"
# Only necessary if using Protobuf well-known types:
prost-types = "0.6"
```

The recommended way to add `.proto` compilation to a Cargo project is to use the
`prost-build` library. See the [`prost-build` documentation](prost-build) for
more details and examples.

## Generated Code

`prost` generates Rust code from source `.proto` files using the `proto2` or
`proto3` syntax. `prost`'s goal is to make the generated code as simple as
possible.

### Packages

All `.proto` files used with `prost` must contain a
[`package` specifier][package]. `prost` will translate the Protobuf package into
a Rust module. For example, given the `package` specifier:

[package]: https://developers.google.com/protocol-buffers/docs/proto#packages

```proto
package foo.bar;
```

All Rust types generated from the file will be in the `foo::bar` module.

### Messages

Given a simple message declaration:

```proto
// Sample message.
message Foo {
}
```

`prost` will generate the following Rust struct:

```rust
/// Sample message.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct Foo {
}
```

### Fields

Fields in Protobuf messages are translated into Rust as public struct fields of the
corresponding type.

#### Scalar Values

Scalar value types are converted as follows:

| Protobuf Type | Rust Type |
| --- | --- |
| `double` | `f64` |
| `float` | `f32` |
| `int32` | `i32` |
| `int64` | `i64` |
| `uint32` | `u32` |
| `uint64` | `u64` |
| `sint32` | `i32` |
| `sint64` | `i64` |
| `fixed32` | `u32` |
| `fixed64` | `u64` |
| `sfixed32` | `i32` |
| `sfixed64` | `i64` |
| `bool` | `bool` |
| `string` | `String` |
| `bytes` | `Vec<u8>` |

#### Enumerations

All `.proto` enumeration types convert to the Rust `i32` type. Additionally,
each enumeration type gets a corresponding Rust `enum` type, with helper methods
to convert `i32` values to the enum type. The `enum` type isn't used directly as
a field, because the Protobuf spec mandates that enumerations values are 'open',
and decoding unrecognized enumeration values must be possible.

#### Field Modifiers

Protobuf scalar value and enumeration message fields can have a modifier
depending on the Protobuf version. Modifiers change the corresponding type of
the Rust field:

| `.proto` Version | Modifier | Rust Type |
| --- | --- | --- |
| `proto2` | `optional` | `Option<T>` |
| `proto2` | `required` | `T` |
| `proto3` | default | `T` |
| `proto2`/`proto3` | repeated | `Vec<T>` |

#### Map Fields

Map fields are converted to a Rust `HashMap` with key and value type converted
from the Protobuf key and value types.

#### Message Fields

Message fields are converted to the corresponding struct type. The table of
field modifiers above applies to message fields, except that `proto3` message
fields without a modifier (the default) will be wrapped in an `Option`.
Typically message fields are unboxed. `prost` will automatically box a message
field if the field type and the parent type are recursively nested in order to
avoid an infinite sized struct.

#### Oneof Fields

Oneof fields convert to a Rust enum. Protobuf `oneof`s types are not named, so
`prost` uses the name of the `oneof` field for the resulting Rust enum, and
defines the enum in a module under the struct. For example, a `proto3` message
such as:

```proto
message Foo {
  oneof widget {
    int32 quux = 1;
    string bar = 2;
  }
}
```

generates the following Rust[1]:

```rust
pub struct Foo {
    pub widget: Option<foo::Widget>,
}
pub mod foo {
    pub enum Widget {
        Quux(i32),
        Bar(String),
    }
}
```

`oneof` fields are always wrapped in an `Option`.

[1] Annotations have been elided for clarity. See below for a full example.

### Services

`prost-build` allows a custom code-generator to be used for processing `service`
definitions. This can be used to output Rust traits according to an
application's specific needs.

### Generated Code Example

Example `.proto` file:

```proto
syntax = "proto3";
package tutorial;

message Person {
  string name = 1;
  int32 id = 2;  // Unique ID number for this person.
  string email = 3;

  enum PhoneType {
    MOBILE = 0;
    HOME = 1;
    WORK = 2;
  }

  message PhoneNumber {
    string number = 1;
    PhoneType type = 2;
  }

  repeated PhoneNumber phones = 4;
}

// Our address book file is just one of these.
message AddressBook {
  repeated Person people = 1;
}
```

and the generated Rust code (`tutorial.rs`):

```rust
#[derive(Clone, Debug, PartialEq, Message)]
#[prost(package=tutorial)]
pub struct Person {
    #[prost(string, tag="1")]
    pub name: String,
    /// Unique ID number for this person.
    #[prost(int32, tag="2")]
    pub id: i32,
    #[prost(string, tag="3")]
    pub email: String,
    #[prost(message, repeated, tag="4")]
    pub phones: Vec<person::PhoneNumber>,
}
pub mod person {
    #[derive(Clone, Debug, PartialEq, Message)]
    #[prost(package=tutorial)]
    pub struct PhoneNumber {
        #[prost(string, tag="1")]
        pub number: String,
        #[prost(enumeration="PhoneType", tag="2")]
        pub type_: i32,
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Enumeration)]
    pub enum PhoneType {
        Mobile = 0,
        Home = 1,
        Work = 2,
    }
}
/// Our address book file is just one of these.
#[derive(Clone, Debug, PartialEq, Message)]
#[prost(package=tutorial)]
pub struct AddressBook {
    #[prost(message, repeated, tag="1")]
    pub people: Vec<Person>,
}
```

## Serializing Existing Types

`prost` uses a custom derive macro to handle encoding and decoding types, which
means that if your existing Rust type is compatible with Protobuf types, you can
serialize and deserialize it by adding the appropriate derive and field
annotations.

Currently the best documentation on adding annotations is to look at the
generated code examples above.

### Tag Inference for Existing Types

Prost automatically infers tags for the struct.

Fields are tagged sequentially in the order they
are specified, starting with `1`.

You may skip tags which have been reserved, or where there are gaps between
sequentially occurring tag values by specifying the tag number to skip to with
the `tag` attribute on the first field after the gap. The following fields will
be tagged sequentially starting from the next number.

```rust
#[derive(Clone, Debug, PartialEq, Message)]
#[prost(package=tutorial)]
struct Person {
  pub id: String, // tag=1

  // NOTE: Old "name" field has been removed
  // pub name: String, // tag=2 (Removed)

  #[prost(tag="6")]
  pub given_name: String, // tag=6
  pub family_name: String, // tag=7
  pub formatted_name: String, // tag=8

  #[prost(tag="3")]
  pub age: u32, // tag=3
  pub height: u32, // tag=4
  #[prost(enumeration="Gender")]
  pub gender: i32, // tag=5

  // NOTE: Skip to less commonly occurring fields
  #[prost(tag="16")]
  pub name_prefix: String, // tag=16  (eg. mr/mrs/ms)
  pub name_suffix: String, // tag=17  (eg. jr/esq)
  pub maiden_name: String, // tag=18
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Enumeration)]
pub enum Gender {
  Unknown = 0,
  Female = 1,
  Male = 2,
}
```

## Well Known Types

To use the well known types such as `Any` and `Value`, you will have to include
prost-types in your dependencies:
```
[dependencies]
prost = <prost-version>
bytes = <bytes-version>
prost-types = <prost-version>
```

Currently Prost has convenience methods for wrapping common Rust types into `Value`
types:

```rust
use prost_types::Value;

#[test]
fn test_well_known_types_value() {
    let number: Value = Value::from(10.0);
    let null: Value = Value::null();
    let string: Value = Value::from(String::from("Hello"));
    let list = vec![Value::null(), Value::from(100.0)];
    let pb_list: Value = Value::from(list);
    let mut map: BTreeMap<String, Value> = BTreeMap::new();
    map.insert(String::from("number"), number);
    map.insert(String::from("null"), null);
    map.insert(String::from("string"), string);
    map.insert(String::from("list"), pb_list);
    let pb_struct: Value = Value::from(map);
    println!("Struct: {:?}", pb_struct);
}
```

Converting from `Value` back into its Rust value can be done by using the `TryFrom`
trait that is implemented on all `Value` types, e.g.:

```rust
#[test]
fn test_well_known_types_convert_number() {
    let number: Value = Value::from(10.0);
    let back: f64 = number.try_into().unwrap();
    assert_eq!(10.0, back)
}
```

The "packing" and "unpacking" of messages into the `Any` value is also supported:

```rust
use prost_types::Any;

#[test]
fn test_well_known_types_any() {
    let msg = Foo {
        null: ::prost_types::NullValue::NullValue.into(),
        timestamp: Some(::prost_types::Timestamp {
            seconds: 99,
            nanos: 42,
        }),
    };
    let any = Any::pack(msg);
    println!("{:?}", any);
    let unpacked = any.unpack(Foo::default()).unwrap();
    println!("{:?}", unpacked);
}
```

If you want to include JSON serialization and deserialization support for
the well-known-types, you can do so by setting the `include_serde` option
in the prost-build `build.rs`:

```rust
fn main() {
    let mut prost_build = prost_build::Config::new();
    prost_build.include_serde();
    prost_build.compile_protos(&["src/frontend.proto", "src/backend.proto"],
                               &["src"]).unwrap();
}
```

Setting this option will decorate a `Message` struct as follows:
```rust
#[derive(Clone, Debug, PartialEq, ::prost::Message)]
#[prost(package=tutorial)]
#[derive(Serialize, Deserialize)]
#[prost(serde)]
#[serde(default, rename_all="camelCase")]
pub struct AddressBook {
    #[prost(message, repeated, tag="1")]
    pub people: Vec<Person>,
}
```

Doing so will allow you to _pack_ a `Message` struct into an `Any` and
serialize it properly to JSON, e.g.:
```rust
use prost::*;
use serde::{Serialize, Deserialize};

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
    println!("{}", json);
    let back: Foo = serde_json::from_str(&json).unwrap();
    println!("{:?}", &back);
    assert_eq!(back, original)
}
```

You will also be able to deserialize any message packed in an Any struct
properly:

```rust
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
```

## FAQ

1. **Could `prost` be implemented as a serializer for [Serde](https://serde.rs/)?**

  Probably not, however I would like to hear from a Serde expert on the matter.
  There are two complications with trying to serialize Protobuf messages with
  Serde:

  - Protobuf fields require a numbered tag, and curently there appears to be no
    mechanism suitable for this in `serde`.
  - The mapping of Protobuf type to Rust type is not 1-to-1. As a result,
    trait-based approaches to dispatching don't work very well. Example: six
    different Protobuf field types correspond to a Rust `Vec<i32>`: `repeated
    int32`, `repeated sint32`, `repeated sfixed32`, and their packed
    counterparts.

  But it is possible to place `serde` derive tags onto the generated types, so
  the same structure can support both `prost` and `Serde`.

2. **I get errors when trying to run `cargo test` on MacOS**

  If the errors are about missing `autoreconf` or similar, you can probably fix
  them by running

  ```
  brew install automake
  brew install libtool
  ```

## License

`prost` is distributed under the terms of the Apache License (Version 2.0).

See [LICENSE](LICENSE) for details.

Copyright 2017 Dan Burkert
