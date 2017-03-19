#![feature(test)]

extern crate proto;
extern crate test;
extern crate bytes;

use test::Bencher;

use bytes::{
    BytesMut,
    IntoBuf,
};

use proto::field::{
    encode_varint,
    decode_varint,
    varint_width
};

macro_rules! varint_bench {
    ($encode_name:ident, $decode_name:ident, $expected_bytes:expr, $encode:expr) => {
        #[bench]
        fn $encode_name(b: &mut Bencher) {
            let mut buf = BytesMut::with_capacity(4096);
            b.iter(|| {
                buf.clear();
                $encode(&mut buf);
                test::black_box(&buf[..]);
            });
            assert_eq!(buf.len(), $expected_bytes);
            b.bytes = 100 * 8;
        }
        #[bench]
        fn $decode_name(b: &mut Bencher) {
            let mut buf = BytesMut::with_capacity(4096);
            $encode(&mut buf);
            let buf = buf.freeze();

            let mut values = [0u64; 100];

            b.iter(|| {
                let mut buf = buf.clone().into_buf();
                for i in 0..100 {
                    values[i] = decode_varint(&mut buf).unwrap();
                }
                test::black_box(&values[..]);
            });
            b.bytes = 100 * 8;
        }
    }
}

/// Benchmark encoding and decoding 100 varints of mixed width (average 5.5 bytes).
varint_bench!(encode_varint_mixed, decode_varint_mixed, 550, |ref mut buf| {
    for width in 0..10 {
        let exponent = width * 7;
        for offset in 0..10 {
            encode_varint(offset + (1 << exponent), buf);
        }
    }
});

/// Benchmark encoding and decoding 100 small (1 byte) varints.
varint_bench!(encode_varint_small, decode_varint_small, 100, |ref mut buf| {
    for value in 0..100 {
        encode_varint(value, buf);
    }
});

/// Benchmark encoding and decoding 100 medium (5 byte) varints.
varint_bench!(encode_varint_medium, decode_varint_medium, 500, |ref mut buf| {
    let start = 1 << 28;
    for value in start..start + 100 {
        encode_varint(value, buf);
    }
});

/// Benchmark encoding and decoding 100 large (10 byte) varints.
varint_bench!(encode_varint_large, decode_varint_large, 1000, |ref mut buf| {
    let start = 1 << 63;
    for value in start..start + 100 {
        encode_varint(value, buf);
    }
});
