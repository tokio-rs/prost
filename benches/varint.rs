#![feature(test)]

extern crate proto;
extern crate test;

use proto::encodable::encode_varint;

use test::Bencher;

/// Benchmark encoding 100 varints of mixed width (average 5.5 bytes).
#[bench]
fn encode_varint_mixed(b: &mut Bencher) {
    let mut buf = Vec::with_capacity(4096);
    b.iter(|| {
        buf.clear();
        for width in 0..10 {
            let exponent = width * 7;
            for offset in 0..10 {
                encode_varint(offset + (1 << exponent), &mut buf);
            }
        }
        test::black_box(&buf[..]);
    });
    assert_eq!(buf.len(), 550);
    b.bytes = 100 * 8;
}

/// Benchmark encoding 100 small (1 byte) varints.
#[bench]
fn encode_varint_small(b: &mut Bencher) {
    let mut buf = Vec::with_capacity(4096);
    b.iter(|| {
        buf.clear();
        for value in 0..100 {
            encode_varint(value, &mut buf);
        }
        test::black_box(&buf[..]);
    });
    assert_eq!(buf.len(), 100);
    b.bytes = 100 * 8;
}

/// Benchmark encoding 100 medium (4 byte) varints.
#[bench]
fn encode_varint_medium(b: &mut Bencher) {
    let mut buf = Vec::with_capacity(4096);
    b.iter(|| {
        buf.clear();
        let start = 1 << 28;
        for value in start..start + 100 {
            encode_varint(value, &mut buf);
        }
        test::black_box(&buf[..]);
    });
    assert_eq!(buf.len(), 5 * 100);
    b.bytes = 100 * 8;
}

/// Benchmark encoding 100 large (10 byte) varints.
#[bench]
fn encode_varint_large(b: &mut Bencher) {
    let mut buf = Vec::with_capacity(4096);
    b.iter(|| {
        buf.clear();
        let start = 1 << 63;
        for value in start..start + 100 {
            encode_varint(value, &mut buf);
        }
        test::black_box(&buf[..]);
    });
    assert_eq!(buf.len(), 10 * 100);
    b.bytes = 100 * 8;
}
