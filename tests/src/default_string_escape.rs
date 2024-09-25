include!(concat!(env!("OUT_DIR"), "/default_string_escape.rs"));

#[test]
fn test_default_string_escape() {
    let msg = Person::default();
    assert_eq!(msg.name, r#"["unknown"]"#);
}
