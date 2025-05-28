use prost::Message;

#[cfg(not(feature = "std"))]
use alloc::{string::ToString, vec};

mod default_types {
    include!(concat!(
        env!("OUT_DIR"),
        "/force_required_messages_default.rs"
    ));
}

mod required_types {
    include!(concat!(
        env!("OUT_DIR"),
        "/force_required_messages_required.rs"
    ));
}

#[test]
fn test_marshalling_unmarshalling_default_vs_required() {
    let default_person = default_types::Person {
        name: "John Doe".to_string(),
        age: 30,
        home_address: Some(default_types::Address {
            street: "123 Main St".to_string(),
            city: "Anytown".to_string(),
            country: "USA".to_string(),
        }),
        contact_info: Some(default_types::Contact {
            email: "john@example.com".to_string(),
            phone: "+1-555-0123".to_string(),
        }),
        work_address: Some(default_types::Address {
            street: "456 Work Ave".to_string(),
            city: "Business City".to_string(),
            country: "USA".to_string(),
        }),
        emergency_contact: None,
        previous_addresses: vec![],
        additional_contacts: vec![],
    };

    let required_person = required_types::Person {
        name: "John Doe".to_string(),
        age: 30,
        home_address: required_types::Address {
            street: "123 Main St".to_string(),
            city: "Anytown".to_string(),
            country: "USA".to_string(),
        },
        contact_info: required_types::Contact {
            email: "john@example.com".to_string(),
            phone: "+1-555-0123".to_string(),
        },
        work_address: Some(required_types::Address {
            street: "456 Work Ave".to_string(),
            city: "Business City".to_string(),
            country: "USA".to_string(),
        }),
        emergency_contact: None,
        previous_addresses: vec![],
        additional_contacts: vec![],
    };

    let default_encoded = default_person.encode_to_vec();
    let required_encoded = required_person.encode_to_vec();

    assert_eq!(
        default_encoded, required_encoded,
        "Encoded bytes should be identical when all fields are populated"
    );

    let required_from_default = required_types::Person::decode(default_encoded.as_slice()).unwrap();
    let default_from_required = default_types::Person::decode(required_encoded.as_slice()).unwrap();

    assert_eq!(required_from_default.name, required_person.name);
    assert_eq!(required_from_default.age, required_person.age);
    assert_eq!(
        required_from_default.home_address,
        required_person.home_address
    );
    assert_eq!(
        required_from_default.contact_info,
        required_person.contact_info
    );

    assert_eq!(default_from_required.name, default_person.name);
    assert_eq!(default_from_required.age, default_person.age);
    assert_eq!(
        default_from_required.home_address,
        default_person.home_address
    );
    assert_eq!(
        default_from_required.contact_info,
        default_person.contact_info
    );
}

#[test]
fn test_marshalling_with_missing_fields() {
    let default_person_minimal = default_types::Person {
        name: "Jane Doe".to_string(),
        age: 25,
        home_address: None,
        contact_info: None,
        work_address: None,
        emergency_contact: None,
        previous_addresses: vec![],
        additional_contacts: vec![],
    };

    let required_person_minimal = required_types::Person {
        name: "Jane Doe".to_string(),
        age: 25,
        home_address: required_types::Address::default(),
        contact_info: required_types::Contact::default(),
        work_address: None,
        emergency_contact: None,
        previous_addresses: vec![],
        additional_contacts: vec![],
    };

    let default_encoded = default_person_minimal.encode_to_vec();
    let required_encoded = required_person_minimal.encode_to_vec();

    assert_ne!(
        default_encoded, required_encoded,
        "Encoded bytes should differ when required mode includes default values"
    );

    let default_decoded = default_types::Person::decode(default_encoded.as_slice()).unwrap();
    let required_decoded = required_types::Person::decode(required_encoded.as_slice()).unwrap();

    assert_eq!(default_decoded.home_address, None);
    assert_eq!(default_decoded.contact_info, None);

    assert_eq!(
        required_decoded.home_address,
        required_types::Address::default()
    );
    assert_eq!(
        required_decoded.contact_info,
        required_types::Contact::default()
    );

    let required_from_minimal_default =
        required_types::Person::decode(default_encoded.as_slice()).unwrap();

    assert_eq!(
        required_from_minimal_default.home_address,
        required_types::Address::default()
    );
    assert_eq!(
        required_from_minimal_default.contact_info,
        required_types::Contact::default()
    );

    let default_from_required = default_types::Person::decode(required_encoded.as_slice()).unwrap();

    assert_eq!(
        default_from_required.home_address,
        Some(default_types::Address::default())
    );
    assert_eq!(
        default_from_required.contact_info,
        Some(default_types::Contact::default())
    );
}

#[test]
fn test_company_marshalling() {
    let default_company = default_types::Company {
        name: "Tech Corp".to_string(),
        headquarters: Some(default_types::Address {
            street: "100 Tech Blvd".to_string(),
            city: "Silicon Valley".to_string(),
            country: "USA".to_string(),
        }),
        employees: vec![],
        ceo: None,
    };

    let required_company = required_types::Company {
        name: "Tech Corp".to_string(),
        headquarters: required_types::Address {
            street: "100 Tech Blvd".to_string(),
            city: "Silicon Valley".to_string(),
            country: "USA".to_string(),
        },
        employees: vec![],
        ceo: None,
    };

    let default_encoded = default_company.encode_to_vec();
    let required_encoded = required_company.encode_to_vec();

    assert_eq!(default_encoded, required_encoded);

    let required_from_default =
        required_types::Company::decode(default_encoded.as_slice()).unwrap();
    let default_from_required =
        default_types::Company::decode(required_encoded.as_slice()).unwrap();

    assert_eq!(required_from_default.name, required_company.name);
    assert_eq!(
        required_from_default.headquarters,
        required_company.headquarters
    );
    assert_eq!(required_from_default.ceo, required_company.ceo);

    assert_eq!(default_from_required.name, default_company.name);
    assert_eq!(
        default_from_required.headquarters,
        default_company.headquarters
    );
    assert_eq!(default_from_required.ceo, default_company.ceo);
}

#[test]
fn test_empty_message_handling() {
    let default_empty = default_types::Person::default();
    let required_empty = required_types::Person::default();

    let default_encoded = default_empty.encode_to_vec();
    let required_encoded = required_empty.encode_to_vec();

    assert_ne!(default_encoded, required_encoded);

    assert_eq!(default_empty.home_address, None);
    assert_eq!(default_empty.contact_info, None);

    assert_eq!(
        required_empty.home_address,
        required_types::Address::default()
    );
    assert_eq!(
        required_empty.contact_info,
        required_types::Contact::default()
    );

    let empty_bytes = vec![];

    let default_from_empty = default_types::Person::decode(empty_bytes.as_slice()).unwrap();
    let required_from_empty = required_types::Person::decode(empty_bytes.as_slice()).unwrap();

    assert_eq!(default_from_empty.home_address, None);
    assert_eq!(default_from_empty.contact_info, None);

    assert_eq!(
        required_from_empty.home_address,
        required_types::Address::default()
    );
    assert_eq!(
        required_from_empty.contact_info,
        required_types::Contact::default()
    );
}

#[test]
fn test_roundtrip_consistency() {
    let test_cases = vec![(
        default_types::Person {
            name: "Alice".to_string(),
            age: 28,
            home_address: Some(default_types::Address {
                street: "789 Oak St".to_string(),
                city: "Hometown".to_string(),
                country: "Canada".to_string(),
            }),
            contact_info: Some(default_types::Contact {
                email: "alice@test.com".to_string(),
                phone: "+1-555-9876".to_string(),
            }),
            work_address: None,
            emergency_contact: None,
            previous_addresses: vec![],
            additional_contacts: vec![],
        },
        required_types::Person {
            name: "Alice".to_string(),
            age: 28,
            home_address: required_types::Address {
                street: "789 Oak St".to_string(),
                city: "Hometown".to_string(),
                country: "Canada".to_string(),
            },
            contact_info: required_types::Contact {
                email: "alice@test.com".to_string(),
                phone: "+1-555-9876".to_string(),
            },
            work_address: None,
            emergency_contact: None,
            previous_addresses: vec![],
            additional_contacts: vec![],
        },
    )];

    for (default_msg, required_msg) in test_cases {
        let default_encoded = default_msg.encode_to_vec();
        let default_decoded = default_types::Person::decode(default_encoded.as_slice()).unwrap();
        let default_re_encoded = default_decoded.encode_to_vec();
        assert_eq!(
            default_encoded, default_re_encoded,
            "Default type roundtrip should be consistent"
        );

        let required_encoded = required_msg.encode_to_vec();
        let required_decoded = required_types::Person::decode(required_encoded.as_slice()).unwrap();
        let required_re_encoded = required_decoded.encode_to_vec();
        assert_eq!(
            required_encoded, required_re_encoded,
            "Required type roundtrip should be consistent"
        );

        let required_from_default =
            required_types::Person::decode(default_encoded.as_slice()).unwrap();
        let default_from_required =
            default_types::Person::decode(required_encoded.as_slice()).unwrap();

        let cross_encoded_1 = required_from_default.encode_to_vec();
        let cross_encoded_2 = default_from_required.encode_to_vec();

        assert_eq!(default_encoded, required_encoded);
        assert_eq!(cross_encoded_1, required_encoded);
        assert_eq!(cross_encoded_2, default_encoded);
    }
}
