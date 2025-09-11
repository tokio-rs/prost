// This test ensures we can compile using re-exported dependencies as configured in
// `build.rs`. Note that there's no direct dependency of `::prost` or `::prost-types` in
// `Cargo.toml`.
include!(concat!(env!("OUT_DIR"), "/prost_path.rs"));

#[test]
fn type_can_be_constructed() {
    use reexported_prost::prost_types::value::Kind;
    use reexported_prost::prost_types::{Timestamp, Value};

    use self::msg::C;

    let _msg = Msg {
        a: 1,
        b: "test".to_string(),
        timestamp: Some(Timestamp {
            nanos: 3,
            seconds: 3,
        }),
        value: Some(Value {
            kind: Some(Kind::BoolValue(true)),
        }),
        c: Some(C::D(1)),
    };
}
