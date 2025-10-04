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

    let _ = oneof_name_conflict::EnumAndOneofConflict {
        r#type: Some(
            oneof_name_conflict::enum_and_oneof_conflict::TypeOneOf::TypeThree(
                oneof_name_conflict::enum_and_oneof_conflict::TypeThree {
                    field: oneof_name_conflict::enum_and_oneof_conflict::Type::Type1.into(),
                },
            ),
        ),
    };

    let _ = oneof_name_conflict::NestedTypeWithReservedKeyword {
        r#abstract: Some(
            oneof_name_conflict::nested_type_with_reserved_keyword::AbstractOneOf::Abstract(
                oneof_name_conflict::nested_type_with_reserved_keyword::Abstract {
                    field: "field".into(),
                },
            ),
        ),
    };
}
