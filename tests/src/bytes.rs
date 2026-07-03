use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use prost::bytes::Bytes;
use prost::encoding::wire_type::WireType;
use prost::encoding::DecodeContext;

#[derive(Clone, prost::Message)]
struct MessageWithBytes {
    #[prost(bytes = "vec", tag = "1")]
    legacy_vec: Vec<u8>,
    #[prost(bytes = "vec", optional, tag = "2")]
    legacy_opt_vec: Option<Vec<u8>>,
    #[prost(bytes = "vec", repeated, tag = "3")]
    legacy_rep_vec: Vec<Vec<u8>>,

    #[prost(bytes = "bytes", tag = "4")]
    legacy_bytes: Bytes,
    #[prost(bytes = "bytes", optional, tag = "5")]
    legacy_opt_bytes: Option<Bytes>,
    #[prost(bytes = "bytes", repeated, tag = "6")]
    legacy_rep_bytes: Vec<Bytes>,

    #[prost(bytes, tag = "11")]
    vec_default: Vec<u8>,
    #[prost(bytes, optional, tag = "12")]
    opt_vec_default: Option<Vec<u8>>,
    #[prost(bytes, repeated, tag = "13")]
    rep_vec_default: Vec<Vec<u8>>,

    #[prost(bytes, encoding = "VecU8Encoding", tag = "14")]
    vec: Vec<u8>,
    #[prost(bytes, encoding = "VecU8Encoding", optional, tag = "15")]
    opt_vec: Option<Vec<u8>>,
    #[prost(bytes, encoding = "VecU8Encoding", repeated, tag = "16")]
    rep_vec: Vec<Vec<u8>>,

    #[prost(bytes, encoding = "BytesEncoding", tag = "17")]
    bytes: Bytes,
    #[prost(bytes, encoding = "BytesEncoding", optional, tag = "18")]
    opt_bytes: Option<Bytes>,
    #[prost(bytes, encoding = "BytesEncoding", repeated, tag = "19")]
    rep_bytes: Vec<Bytes>,

    #[prost(btree_map = "uint32, bytes", tag = "21")]
    map_vec_default: BTreeMap<u32, Vec<u8>>,
    #[prost(
        btree_map = "uint32, bytes",
        value_encoding = "VecU8Encoding",
        tag = "22"
    )]
    map_vec: BTreeMap<u32, Vec<u8>>,
    #[prost(
        btree_map = "uint32, bytes",
        value_encoding = "BytesEncoding",
        tag = "23"
    )]
    map_bytes: BTreeMap<u32, Bytes>,

    #[prost(
        bytes,
        encoding = "MyBytesEncoding",
        encoding_module = "self",
        tag = "31"
    )]
    my_bytes: MyBytes,
    #[prost(
        bytes,
        encoding = "MyBytesEncoding",
        encoding_module = "self",
        optional,
        tag = "32"
    )]
    opt_my_bytes: Option<MyBytes>,
    #[prost(
        bytes,
        encoding = "MyBytesEncoding",
        encoding_module = "crate::bytes",
        repeated,
        tag = "33"
    )]
    rep_my_bytes: Vec<MyBytes>,
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
struct MyBytes(Vec<u8>);

impl core::ops::Deref for MyBytes {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

impl MyBytes {
    fn clear(&mut self) {
        self.0.clear();
    }
}

impl PartialEq<&[u8]> for MyBytes {
    fn eq(&self, other: &&[u8]) -> bool {
        self.0.as_slice() == *other
    }
}

struct MyBytesEncoding;

impl prost::encoding::Encoding for MyBytesEncoding {
    type Type = MyBytes;

    fn encoded_len(tag: u32, value: &Self::Type) -> usize {
        prost::encoding::bytes::encoded_len(tag, &value.0)
    }

    fn encode(tag: u32, value: &Self::Type, buf: &mut impl prost::bytes::BufMut) {
        prost::encoding::bytes::encode(tag, &value.0, buf);
    }

    fn merge<B: prost::bytes::Buf>(
        wire_type: WireType,
        value: &mut Self::Type,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<(), prost::DecodeError> {
        prost::encoding::bytes::merge(wire_type, &mut value.0, buf, ctx)
    }
}

// No actual tests: This test that the prost-derive attributes works
