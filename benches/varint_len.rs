use criterion::{Criterion, criterion_group, criterion_main, black_box};

#[inline(always)]
fn varint_len(v: u32) -> u32 {
    match v {
        0..=127 => 1,
        128..=16383 => 2,
        16384..=2097151 => 3,
        2097152..=268435456 => 4,
        _ => 5
    }
}

#[inline(always)]
fn varint_len_loop(mut v: u32) -> u32 {
    let mut length = 1;

    while v > 127 {
        v >>= 7;
		length += 1;
    }

    length
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("varint length");

    group.bench_function("match", |b| {
        //let mut range = (0..=u32::MAX).cycle();

        b.iter(|| varint_len(black_box(i32::MAX as u32)));
    });
    group.bench_function("loop", |b| {
        //let mut range = (0..=u32::MAX).cycle();

        b.iter(|| varint_len_loop(black_box(i32::MAX as u32)));
    });
    group.finish()
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);