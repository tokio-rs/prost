use std::hint::black_box;

use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, BenchmarkId, Criterion,
    Throughput,
};
use prost::Message;

#[derive(PartialEq, Message)]
struct Node {
    #[prost(uint64, tag = "1")]
    id: u64,
    #[prost(string, tag = "2")]
    label: String,
    #[prost(bytes = "vec", tag = "3")]
    payload: Vec<u8>,
    #[prost(uint64, repeated, packed = "true", tag = "4")]
    values: Vec<u64>,
    #[prost(message, repeated, tag = "5")]
    children: Vec<Node>,
}

fn node(id: &mut u64, children: Vec<Node>) -> Node {
    *id += 1;
    Node {
        id: *id,
        label: format!("{:016x}", *id),
        payload: vec![(*id % 251) as u8; 32],
        values: vec![1, 127, 128, 16_384, *id * 101, 1 << 31, 1 << 56, u64::MAX],
        children,
    }
}

// Depth counts the root: depth 1 is a single node without children.
fn balanced(id: &mut u64, depth: usize, fanout: usize) -> Node {
    assert!(depth >= 1);
    let children = if depth == 1 {
        Vec::new()
    } else {
        (0..fanout)
            .map(|_| balanced(id, depth - 1, fanout))
            .collect()
    };
    node(id, children)
}

// A chain ending in a parent with all remaining nodes as leaves keeps the
// scalar work and node count constant while varying the nesting depth.
fn fixed_nodes(total: usize, depth: usize) -> Node {
    assert!(depth >= 2 && total >= depth);
    let mut id = 0;
    let leaves = (0..total - depth + 1)
        .map(|_| node(&mut id, Vec::new()))
        .collect();
    let mut root = node(&mut id, leaves);
    for _ in 0..depth - 2 {
        root = node(&mut id, vec![root]);
    }
    root
}

fn shape(message: &Node) -> (usize, usize) {
    message
        .children
        .iter()
        .fold((1, 1), |(nodes, depth), child| {
            let (child_nodes, child_depth) = shape(child);
            (nodes + child_nodes, depth.max(child_depth + 1))
        })
}

fn benchmark_message(
    group: &mut BenchmarkGroup<'_, WallTime>,
    name: &str,
    message: Node,
    expected_shape: (usize, usize),
) {
    // Validate every shape outside the timed region, including in Criterion's
    // --test mode. Both feature configurations must produce the raw wire bytes.
    assert_eq!(shape(&message), expected_shape);
    let wire = message.encode_to_vec();
    let size = message.encoded_len();
    assert_eq!(wire.len(), size);
    let mut raw = Vec::with_capacity(size);
    message.encode_raw(&mut raw);
    assert_eq!(wire, raw);
    assert_eq!(Node::decode(wire.as_slice()).unwrap(), message);

    group.throughput(Throughput::Bytes(size as u64));
    group.bench_with_input(
        BenchmarkId::new("encode_new", name),
        &message,
        |b, message| {
            b.iter(|| {
                // Includes the size pass, output allocation, encoding, and drop.
                black_box(black_box(message).encode_to_vec());
            });
        },
    );
    group.bench_with_input(
        BenchmarkId::new("encode_reuse", name),
        &message,
        |b, message| {
            let mut output = Vec::with_capacity(size);
            message.encode(&mut output).unwrap();
            assert_eq!(output, wire);
            b.iter(|| {
                output.clear();
                // Reuse output capacity, but recompute all lengths each time.
                black_box(message).encode(&mut output).unwrap();
                black_box(&output);
            });
        },
    );
}

fn nested(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("nested");

    // No active nested fields: control for the TLS bypass.
    benchmark_message(&mut group, "flat", node(&mut 0, Vec::new()), (1, 1));

    // Match node count and scalar work to isolate the effect of depth.
    for depth in [2, 64] {
        benchmark_message(
            &mut group,
            &format!("fixed1023_d{depth}"),
            fixed_nodes(1023, depth),
            (1023, depth),
        );
    }

    // The same 1,023 nodes arranged as a balanced binary tree.
    benchmark_message(
        &mut group,
        "balanced_f2_d10",
        balanced(&mut 0, 10, 2),
        (1023, 10),
    );
    group.finish();
}

criterion_group!(benches, nested);
criterion_main!(benches);
