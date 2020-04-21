use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use criterion::{criterion_group, criterion_main, Criterion};
use prost::Message;

use protobuf::benchmarks::{
    dataset, google_message3::GoogleMessage3, /*google_message4::GoogleMessage4,*/ proto2,
    proto3, BenchmarkDataset,
};

fn load_dataset(dataset: &Path) -> Result<BenchmarkDataset, Box<dyn Error>> {
    let mut f = File::open(dataset)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    Ok(BenchmarkDataset::decode(&*buf)?)
}

fn benchmark_dataset<M>(criterion: &mut Criterion, name: &str, dataset: &'static Path)
where
    M: prost::Message + Default + 'static,
{
    criterion.bench_function(&format!("dataset/{}/merge", name), move |b| {
        let dataset = load_dataset(dataset).unwrap();
        let mut message = M::default();
        b.iter(|| {
            for buf in &dataset.payload {
                message.clear();
                message.merge(buf.as_slice()).unwrap();
                criterion::black_box(&message);
            }
        });
    });

    criterion.bench_function(&format!("dataset/{}/encode", name), move |b| {
        let messages = load_dataset(dataset)
            .unwrap()
            .payload
            .iter()
            .map(Vec::as_slice)
            .map(M::decode)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let mut buf = Vec::with_capacity(messages.iter().map(M::encoded_len).sum::<usize>());
        b.iter(|| {
            buf.clear();
            for message in &messages {
                message.encode(&mut buf).unwrap();
            }
            criterion::black_box(&buf);
        });
    });

    criterion.bench_function(&format!("dataset/{}/encoded_len", name), move |b| {
        let messages = load_dataset(dataset)
            .unwrap()
            .payload
            .iter()
            .map(Vec::as_slice)
            .map(M::decode)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        b.iter(|| {
            let encoded_len = messages.iter().map(M::encoded_len).sum::<usize>();
            criterion::black_box(encoded_len)
        });
    });
}

macro_rules! dataset {
    ($name: ident, $ty: ty) => {
        fn $name(criterion: &mut Criterion) {
            benchmark_dataset::<$ty>(criterion, stringify!($name), dataset::$name());
        }
    };
}

dataset!(google_message1_proto2, proto2::GoogleMessage1);
dataset!(google_message1_proto3, proto3::GoogleMessage1);
dataset!(google_message2, proto2::GoogleMessage2);
//dataset!(google_message3_1, GoogleMessage3);
dataset!(google_message3_2, GoogleMessage3);
dataset!(google_message3_3, GoogleMessage3);
dataset!(google_message3_4, GoogleMessage3);
//dataset!(google_message3_5, GoogleMessage3);
//dataset!(google_message4, GoogleMessage4);

criterion_group!(
    dataset,
    google_message1_proto2,
    google_message1_proto3,
    google_message2,
);

criterion_group! {
    name = slow;
    config = Criterion::default().sample_size(10);
    targets = google_message3_2, google_message3_4, google_message3_3
}

// TODO: Criterion now requires a sample_size of 10; figure out a better way to
// get these tests to run in a reasonable time.
/*
criterion_group! {
    name = extra_slow;
    config = Criterion::default().sample_size(10);
    targets = google_message3_1, google_message3_5, google_message4
}
*/

criterion_main!(dataset, slow /*, extra_slow*/);
