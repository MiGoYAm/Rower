use bytes::{BufMut, BytesMut};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

#[inline(always)]
fn write_21bit_varint(value: u32, buf: &mut BytesMut) {
    let w = (value & 0x7F | 0x80) << 16 | ((value >> 7) & 0x7F | 0x80) << 8 | (value >> 14);
        buf.put_u16((w >> 8) as u16);
        buf.put_u8(w as u8);
}

#[inline(always)]
fn write_21bit_varint_bytes(value: u32, buf: &mut BytesMut) {
    buf.put_u16(((value & 0x7F | 0x80) << 8 | ((value >> 7) & 0x7F | 0x80)) as u16);
    buf.put_u8((value >> 14) as u8);
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_varint_21");

    group.bench_function("normal", |b| {
        let mut buf = BytesMut::with_capacity(268435456);
        let mut range = (0..=2_097_151).cycle();

        b.iter_batched(|| range.next().unwrap(), |r| write_21bit_varint(r, &mut buf), BatchSize::LargeInput);
    });
    group.bench_function("bytes", |b| {
        let mut buf = BytesMut::with_capacity(268435456);
        let mut range = (0..=2_097_151).cycle();

        b.iter_batched(|| range.next().unwrap(), |r| write_21bit_varint_bytes(r, &mut buf), BatchSize::LargeInput);
    });

    group.finish()
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
