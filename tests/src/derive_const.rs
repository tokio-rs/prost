use core::fmt::Debug;
use prost::Message;

/// Const array container
#[derive(Clone, PartialEq, Message)]
pub struct TestArray {
    #[prost(message, required, tag = "1")]
    pub b: [u8; 3],
}

#[test]
fn encode_decode_const_array() {
    let t = TestArray {
        b: [1, 2, 3],
    };

    let buff = t.encode_to_vec();

    let b = TestArray::decode(&*buff).unwrap();

    assert_eq!(a, b);

    panic!("whjoops");
}
