use crate::check_message;
use std::fmt::{self, Debug};

#[derive(PartialEq, prost::Message)]
#[prost(debug = false)]
struct NoDebug {
    #[prost(int32, tag = "1")]
    foo: i32,
}

impl Debug for NoDebug {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("NoDebug")
           .field("foo", &self.foo)
           .finish()
    }
}

#[derive(PartialEq, prost::Message)]
#[prost(default = false)]
struct NoDefault {
    #[prost(int32, tag = "1")]
    foo: i32,
    bar: String,
}

impl Default for NoDefault {
    fn default() -> Self {
        NoDefault {
            foo: 0,
            bar: "bar".to_string(),
        }
    }
}

#[test]
fn no_debug() {
    let no_debug = NoDebug::default();
    check_message(&no_debug);

    assert_eq!(format!("{:?}", no_debug), "NoDebug { foo: 0 }");
}

#[test]
fn no_default() {
    let no_default = NoDefault::default();
    check_message(&no_default);

    assert_eq!(format!("{:?}", no_default), "NoDefault { foo: 0 }");
}
