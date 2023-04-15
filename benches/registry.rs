use criterion::{criterion_main, criterion_group, Criterion};


fn criterion_benchmark(_c: &mut Criterion)  {}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);