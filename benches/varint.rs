use bytes::{buf::Chain, Buf, BufMut, BytesMut};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};

#[inline(always)]
fn write_varint_loop(buf: &mut BytesMut, mut value: u32) {
    loop {
        if (value & 0xFFFFFF80) == 0 {
            buf.put_u8(value as u8);
            return;
        }

        buf.put_u8((value & 0x7F | 0x80) as u8);
        value >>= 7;
    }
}
#[inline(always)]
fn write_varint_best(buf: &mut BytesMut, value: u32) {
    if (value & (0xFFFFFFFF << 7)) == 0 {
        buf.put_u8(value as u8);
    } else if (value & (0xFFFFFFFF << 14)) == 0 {
        let w = (value & 0x7F | 0x80) << 8 | (value >> 7);
        buf.put_u16(w as u16);
    } else if (value & (0xFFFFFFFF << 21)) == 0 {
        let w = (value & 0x7F | 0x80) << 16 | ((value >> 7) & 0x7F | 0x80) << 8 | (value >> 14);
        buf.put_u16(w as u16);
        buf.put_u8((w >> 14) as u8);
    } else if (value & (0xFFFFFFFF << 28)) == 0 {
        let w = (value & 0x7F | 0x80) << 24 | (((value >> 7) & 0x7F | 0x80) << 16) | ((value >> 14) & 0x7F | 0x80) << 8 | (value >> 21);
        buf.put_u32(w);
    } else {
        let w = (value & 0x7F | 0x80) << 24 | ((value >> 7) & 0x7F | 0x80) << 16 | ((value >> 14) & 0x7F | 0x80) << 8 | ((value >> 21) & 0x7F | 0x80);
        buf.put_u32(w);
        buf.put_u8((value >> 28) as u8);
    }
}

#[inline(always)]
fn write_varint_best_short(buf: &mut BytesMut, value: u32) {
    if (value & (0xFFFFFFFF << 7)) == 0 {
        buf.put_u8(value as u8);
    } else if (value & (0xFFFFFFFF << 14)) == 0 {
        let w = (value & 0x7F | 0x80) << 8 | (value >> 7);
        buf.put_u16(w as u16);
    } else {
        let w = (value & 0x7F | 0x80) << 16 | ((value >> 7) & 0x7F | 0x80) << 8 | (value >> 14);
        buf.put_u16(w as u16);
        buf.put_u8((w >> 14) as u8);
    }
}

#[inline(always)]
fn extend() -> BytesMut {
    let mut header = BytesMut::with_capacity(2);
    let data = BytesMut::with_capacity(1048576 * 1);
    header.extend_from_slice(&data);
    header
}
#[inline(always)]
fn chain() -> Chain<BytesMut, BytesMut> {
    let header = BytesMut::with_capacity(2);
    let data = BytesMut::with_capacity(1048576 * 1);
    header.chain(data)
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("buf");
    group.bench_function("extend", |b| b.iter(|| extend()));
    group.bench_function("chain", |b| b.iter(|| chain()));
    group.finish();

    let mut group = c.benchmark_group("read varint");

    group.bench_function("normal", |b| {
        let mut buf = BytesMut::with_capacity(268435456);
        let mut range = (0..=2_097_151).cycle();

        b.iter_batched(|| range.next().unwrap(), |r| write_varint_loop(&mut buf, r), BatchSize::LargeInput);
    });
    group.bench_function("best", |b| {
        let mut buf = BytesMut::with_capacity(268435456);
        let mut range = (0..=2_097_151).cycle();

        b.iter_batched(|| range.next().unwrap(), |r| write_varint_best(&mut buf, r), BatchSize::LargeInput);
    });
    group.bench_function("best short", |b| {
        let mut buf = BytesMut::with_capacity(268435456);
        let mut range = (0..=2_097_151).cycle();

        b.iter_batched(|| range.next().unwrap(), |r| write_varint_best_short(&mut buf, r), BatchSize::LargeInput);
    });
    group.finish()

    /*
    let mut group = c.benchmark_group("write varint");

    group.bench_function("normal", |b| {
        let mut buf = BytesMut::with_capacity(268435456);
        let mut range  = (0..=2_097_151).cycle();

        b.iter_batched(
            || range.next().unwrap(),
            |r| write_varint_loop(&mut buf, r),
            BatchSize::LargeInput
        );
    });
    group.bench_function("best", |b| {
        let mut buf = BytesMut::with_capacity(268435456);
        let mut range  = (0..=2_097_151).cycle();

        b.iter_batched(
            || range.next().unwrap(),
            |r| write_varint_best(&mut buf, r),
            BatchSize::LargeInput
        );
    });
    group.bench_function("best short", |b| {
        let mut buf = BytesMut::with_capacity(268435456);
        let mut range  = (0..=2_097_151).cycle();

        b.iter_batched(
            || range.next().unwrap(),
            |r| write_varint_best_short(&mut buf, r),
            BatchSize::LargeInput
        );
    });
    group.finish();
    */
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
