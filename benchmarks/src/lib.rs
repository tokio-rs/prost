#![feature(test)]

extern crate bytes;
extern crate prost;
extern crate test;

#[macro_use]
extern crate prost_derive;

mod varint;

pub mod messages {
    include!(concat!(env!("OUT_DIR"), "/messages.rs"));
}

use bytes::IntoBuf;
use prost::Message;

fn bench_message_encode<M>(b: &mut test::Bencher, message: &M) where M: Message  {
    let encoded_len = message.encoded_len();
    let mut buf = Vec::with_capacity(encoded_len);
    b.iter(|| {
        buf.clear();
        message.encode(&mut buf).unwrap();
        test::black_box(&buf[..]);
    });
    b.bytes = encoded_len as u64;
}

fn bench_message_encode_raw<M>(b: &mut test::Bencher, message: &M) where M: Message  {
    let encoded_len = message.encoded_len();
    let mut buf = Vec::with_capacity(encoded_len);
    b.iter(|| {
        buf.clear();
        message.encode_raw(&mut buf);
        test::black_box(&buf[..]);
    });
    b.bytes = encoded_len as u64;
}

fn bench_message_decode<M>(b: &mut test::Bencher, bytes: &[u8]) where M: Message + Default {
    b.iter(|| {
        test::black_box(M::decode(bytes.into_buf()).unwrap());
    });
    b.bytes = bytes.len() as u64;
}

// Encoded GoogleMessage1 extracted from the protobuf cpp benchmarks.
// https://github.com/google/protobuf/tree/3.3.x/benchmarks
const GOOGLE_MESSAGE1: &'static [u8] = include_bytes!("google_message1.data");

#[bench]
fn google_message1_encode(b: &mut test::Bencher) {
    let message = messages::GoogleMessage1::decode(GOOGLE_MESSAGE1.into_buf()).unwrap();
    bench_message_encode(b, &message);
}

#[bench]
fn google_message1_encode_raw(b: &mut test::Bencher) {
    let message = messages::GoogleMessage1::decode(GOOGLE_MESSAGE1.into_buf()).unwrap();
    bench_message_encode_raw(b, &message);
}

#[bench]
fn google_message1_decode(b: &mut test::Bencher) {
    bench_message_decode::<messages::GoogleMessage1>(b, GOOGLE_MESSAGE1);
}
