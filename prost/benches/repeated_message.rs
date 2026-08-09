use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use prost::Message;

const MESSAGE_COUNTS: &[usize] = &[64, 4_096];

#[derive(Clone, PartialEq, Message)]
struct Nested {
    #[prost(uint32, tag = "1")]
    value: u32,
}

#[derive(Clone, PartialEq, Message)]
struct RepeatedMessages {
    #[prost(message, repeated, tag = "1")]
    values: Vec<Nested>,
}

fn repeated_messages(message_count: usize) -> RepeatedMessages {
    RepeatedMessages {
        values: (0..message_count)
            .map(|value| Nested {
                value: value as u32 + 1,
            })
            .collect(),
    }
}

fn benchmark_decode(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("decode/repeated_messages");

    for &message_count in MESSAGE_COUNTS {
        let expected = repeated_messages(message_count);
        let input = expected.encode_to_vec();
        assert_eq!(
            RepeatedMessages::decode(input.as_slice()).unwrap(),
            expected
        );

        group.throughput(Throughput::Elements(message_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(message_count),
            &input,
            |bench, input| {
                bench.iter(|| {
                    std::hint::black_box(
                        RepeatedMessages::decode(std::hint::black_box(input.as_slice())).unwrap(),
                    )
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_decode);
criterion_main!(benches);
