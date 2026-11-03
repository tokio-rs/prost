use std::{hint::black_box, time::Duration};

use bytes::Buf;
use criterion::{BenchmarkId, Criterion, Throughput};
use prost::encoding::{decode_key, fixed32, fixed64, DecodeContext};

macro_rules! bench_fixed {
    ($c:expr, $module:ident, $ty:ty) => {{
        let mut group = $c.benchmark_group(stringify!($module));
        for len in [0, 1, 16, 1024, 1_000_000] {
            let values: Vec<$ty> = (0..len).map(|i| i as $ty).collect();
            let mut encoded = Vec::new();
            $module::encode_packed(1, &values, &mut encoded);
            if !encoded.is_empty() {
                let mut input = encoded.as_slice();
                let (_, wire) = decode_key(&mut input).unwrap();
                let mut decoded = Vec::new();
                $module::merge_repeated(wire, &mut decoded, &mut input, DecodeContext::default())
                    .unwrap();
                assert_eq!(decoded, values);
                assert!(!input.has_remaining());
            }
            group.throughput(Throughput::Bytes((len * size_of::<$ty>()) as u64));
            group.bench_with_input(BenchmarkId::new("encode", len), &len, |b, _| {
                let mut output = Vec::with_capacity(encoded.len());
                b.iter(|| {
                    output.clear();
                    $module::encode_packed(1, black_box(&values), &mut output);
                    black_box(&output);
                });
            });
            group.bench_with_input(BenchmarkId::new("decode", len), &len, |b, _| {
                let mut output = Vec::with_capacity(len);
                b.iter(|| {
                    output.clear();
                    let mut input = black_box(encoded.as_slice());
                    if input.has_remaining() {
                        let (_, wire) = decode_key(&mut input).unwrap();
                        $module::merge_repeated(
                            wire,
                            &mut output,
                            &mut input,
                            DecodeContext::default(),
                        )
                        .unwrap();
                    }
                    black_box(&output);
                });
            });
        }
        group.finish();
    }};
}

fn main() {
    let mut criterion = Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_millis(200))
        .measurement_time(Duration::from_millis(500))
        .configure_from_args();
    bench_fixed!(criterion, fixed32, u32);
    bench_fixed!(criterion, fixed64, u64);
    criterion.final_summary();
}
