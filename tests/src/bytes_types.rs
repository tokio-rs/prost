include!(concat!(env!("OUT_DIR"), "/bytes_types.rs"));

#[test]
fn test_bytes_types() {
    // Successful compilation means the types and generated code are correct
    let _ = BytesTypes {
        is_vec: ::prost::alloc::vec::Vec::<u8>::default(),
        is_bytes: ::prost::bytes::Bytes::default(),
    };
}
