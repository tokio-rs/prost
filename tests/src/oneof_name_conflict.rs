mod oneof_name_conflict {
    include!(concat!(env!("OUT_DIR"), "/oneof_name_conflict.rs"));
}

#[test]
/// Check naming convention by creating an instance
fn test_creation() {
    let _ = oneof_name_conflict::Bakery {
        bread: Some(oneof_name_conflict::bakery::BreadOneOf::B(
            oneof_name_conflict::bakery::Bread { weight: 12 },
        )),
    };
}
