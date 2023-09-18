use std::collections::HashMap;

use criterion::{criterion_group, criterion_main, Criterion, black_box};
use anyhow::anyhow;

pub type PacketProducer = fn();

pub struct ProtocolRegistryH {
    id_to_packet: HashMap<u8, PacketProducer>,
}

impl ProtocolRegistryH {
    pub fn new() -> Self {
        Self {
            id_to_packet: HashMap::new(),
        }
    }
    #[inline]
    pub fn insert_id_to_packet(&mut self, producer: PacketProducer, id: u8) {
        self.id_to_packet.insert(id, producer);
    }
    #[inline]
    pub fn get_packet(&self, id: u8) -> anyhow::Result<&PacketProducer> {
        self.id_to_packet.get(&id).ok_or(anyhow!("Packet with id {:02X?} does not exist in this state or version", id))
    }
}

pub struct ProtocolRegistry {
    id_to_packeta: [Option<PacketProducer>; 128],
}

impl ProtocolRegistry {
    pub fn new() -> Self {
        Self {
            id_to_packeta: [None; 128]
        }
    }
    #[inline]
    pub fn insert_id_to_packet(&mut self, producer: PacketProducer, id: u8) {
        self.id_to_packeta[id as usize] = Some(producer);
    }
    #[inline]
    pub fn get_packet(&self, id: u8) -> anyhow::Result<&PacketProducer> {
        match self.id_to_packeta.get(id as usize) {
            Some(Some(producer)) => Ok(producer),
            _ => Err(anyhow!("Packet with id {:02X?} does not exist in this state or version", id))
        }
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("registry");

    let mut a = ProtocolRegistry::new();
    let mut h = ProtocolRegistryH::new();

    group.bench_function("array insert", |b| {
        b.iter(|| a.insert_id_to_packet(black_box(|| {}), black_box(64)))
    });
    group.bench_function("hash insert", |b| {
        b.iter(|| h.insert_id_to_packet(black_box(|| {}), black_box(64)))
    });

    group.bench_function("array get", |b| {
        b.iter(|| a.get_packet(black_box(64)))
    });
    group.bench_function("hash get", |b| {
        b.iter(|| h.get_packet(black_box(64)))
    });
    group.finish()
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
