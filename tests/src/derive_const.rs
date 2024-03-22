use prost::{encoding::BytesAdapter, Message};

// NOTE: [Message] still requires [Default], which is not implemented for [u8; N],
// so this will only work directly for [u8; <=32] for the moment...
// see: https://github.com/rust-lang/rust/issues/61415

/// Const array container A
#[derive(Clone, PartialEq, Message)]
pub struct TestA {
    #[prost(bytes, required, tag = "1")]
    pub b: [u8; 3],
}

/// Const array container B
#[derive(Clone, PartialEq, Message)]
pub struct TestB {
    #[prost(bytes, required, tag = "1")]
    pub b: [u8; 4],
}

// Test valid encode/decode
#[test]
fn const_array_encode_decode() {
    let t = TestA { b: [1, 2, 3] };

    let buff = t.encode_to_vec();

    let t1 = TestA::decode(&*buff).unwrap();

    assert_eq!(t, t1);
}

// test encode/decode length mismatch
#[test]
fn const_array_length_mismatch() {
    let t = TestA { b: [1, 2, 3] };

    let buff = t.encode_to_vec();

    assert!(TestB::decode(&*buff).is_err());
}
