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

pub const V1_19_4: i32 = 762;
pub const V1_19_3: i32 = 761;
pub const V1_19_1: i32 = 760;
pub const V1_19: i32 = 759;
pub const V1_18_2: i32 = 758;
pub const V1_18: i32 = 757;
pub const V1_17_1: i32 = 756;
pub const V1_17: i32 = 755;
pub const V1_16_4: i32 = 754;
pub const V1_16_3: i32 = 753;
pub const V1_16_2: i32 = 751;
pub const V1_16_1: i32 = 736;
pub const V1_16: i32 = 735;
pub const V1_15_2: i32 = 578;
pub const V1_15_1: i32 = 575;
pub const V1_15: i32 = 573;
pub const V1_14_4: i32 = 498;
pub const V1_14_3: i32 = 490;
pub const V1_14_2: i32 = 485;
pub const V1_14_1: i32 = 480;
pub const V1_14: i32 = 477;
pub const V1_13_2: i32 = 404;
pub const V1_13_1: i32 = 401;
pub const V1_13: i32 = 393;
pub const V1_12_2: i32 = 340;
pub const V1_12_1: i32 = 338;
pub const V1_12: i32 = 335;
pub const V1_11_1: i32 = 316;
pub const V1_11: i32 = 315;
pub const V1_10: i32 = 210;
pub const V1_9_4: i32 = 110;
pub const V1_9_2: i32 = 109;
pub const V1_9_1: i32 = 108;
pub const V1_9: i32 = 107;
pub const V1_8: i32 = 47;
pub enum State {
    Handshake,
    Status,
    Login,
    Play
}

fn serverbound_id(state: State, version: i32) -> Option<u8> {
    if let State::Play = state {
        match version {
            V1_19_4 | V1_19_1 => Some(0x12),
            V1_19_3 | V1_19 => Some(0x11),
            V1_17..=V1_18_2 | V1_14..=V1_15_2 => Some(0x0f),
            V1_16..=V1_16_4 => Some(0x10),
            V1_13..=V1_13_2 => Some(0x0e),
            V1_12_1..=V1_12_2 | V1_9..=V1_11_1 => Some(0x0b),
            V1_12 => Some(0x0c),
            V1_8 => Some(0x00),
            _ => None
        }
    } else {
        None
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

    group.bench_function("trait get", |b| {
        b.iter(|| serverbound_id(black_box(State::Play), black_box(V1_13_1)))
    });

    group.finish()
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
