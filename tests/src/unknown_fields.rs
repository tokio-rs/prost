include!(concat!(env!("OUT_DIR"), "/unknown_fields.rs"));

#[cfg(feature = "std")]
#[test]
fn test_iter_unknown_fields() {
    use bytes::Bytes;
    use prost::{Message, UnknownField};

    let v2 = V2 {
        a: 12345,
        b: 6,
        c: 7,
        d: "hello".to_owned(),
        unknown_fields: Default::default(),
    };

    let bytes = v2.encode_to_vec();
    let v1 = V1::decode(&*bytes).unwrap();

    let mut fields = v1.unknown_fields.iter();
    assert_eq!(fields.next(), Some((1, &UnknownField::Varint(12345))));
    assert_eq!(fields.next(), Some((2, &UnknownField::ThirtyTwoBit(2))));
    assert_eq!(fields.next(), Some((3, &UnknownField::SixtyFourBit(3))));
    assert_eq!(
        fields.next(),
        Some((
            4,
            &UnknownField::LengthDelimited(Bytes::from(&b"hello"[..]))
        ))
    );
    assert_eq!(fields.next(), None);

    assert_eq!(v2.unknown_fields.iter().count(), 0);
}

#[cfg(feature = "std")]
#[test]
fn test_roundtrip_unknown_fields() {
    use prost::Message;

    let original = V2 {
        a: 12345,
        b: 6,
        c: 7,
        d: "hello".to_owned(),
        unknown_fields: Default::default(),
    };

    let original_bytes = original.encode_to_vec();
    let roundtripped_bytes = V1::decode(&*original_bytes).unwrap().encode_to_vec();

    let roundtripped = V2::decode(&*roundtripped_bytes).unwrap();
    assert_eq!(original, roundtripped);
    assert_eq!(roundtripped.unknown_fields.iter().count(), 0);
}
