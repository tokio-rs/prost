include!(concat!(env!("OUT_DIR"), "/unknown_fields.rs"));

#[cfg(feature = "std")]
#[test]
fn test_iter_unknown_fields() {
    use prost::bytes::Bytes;
    use prost::{Message, UnknownField};

    let v2 = MessageWithData {
        a: 12345,
        b: 6,
        c: 7,
        d: "hello".to_owned(),
        ..Default::default()
    };

    let bytes = v2.encode_to_vec();
    let v1 = BlankMessage::decode(&*bytes).unwrap();

    let mut fields = v1._unknown_fields.iter();
    assert_eq!(fields.next(), Some((1, &UnknownField::Varint(12345))));
    assert_eq!(fields.next(), Some((2, &UnknownField::ThirtyTwoBit(6))));
    assert_eq!(fields.next(), Some((3, &UnknownField::SixtyFourBit(7))));
    assert_eq!(
        fields.next(),
        Some((
            4,
            &UnknownField::LengthDelimited(Bytes::from(&b"hello"[..]))
        ))
    );
    assert_eq!(fields.next(), None);

    assert_eq!(v2._unknown_fields.iter().count(), 0);
}

#[cfg(feature = "std")]
#[test]
fn test_roundtrip_unknown_fields() {
    use prost::Message;

    let original = MessageWithData {
        a: 12345,
        b: 6,
        c: 7,
        d: "hello".to_owned(),
        ..Default::default()
    };

    let original_bytes = original.encode_to_vec();
    let roundtripped_bytes = BlankMessage::decode(&*original_bytes)
        .unwrap()
        .encode_to_vec();

    let roundtripped = MessageWithData::decode(&*roundtripped_bytes).unwrap();
    assert_eq!(original, roundtripped);
    assert_eq!(roundtripped._unknown_fields.iter().count(), 0);
}
