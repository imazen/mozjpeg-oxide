//! Encoding benchmarks for mozjpeg-rs.

use criterion::{criterion_group, criterion_main, Criterion};

fn encode_benchmark(_c: &mut Criterion) {
    // TODO: Add encoding benchmarks once encoder is implemented
}

criterion_group!(benches, encode_benchmark);
criterion_main!(benches);
