use std::{any::TypeId, collections::HashMap};

use anyhow::anyhow;
use bytes::BytesMut;
use once_cell::sync;
use strum::IntoEnumIterator;

use crate::protocol::{
    packet::{
        handshake::Handshake,
        login::{Disconnect, EncryptionRequest, EncryptionResponse, LoginStart, LoginSuccess, SetCompression},
        play::{PluginMessage, JoinGame, Respawn},
        status::{Ping, StatusRequest, StatusResponse},
        Packet, PacketType,
    },
    Direction, ProtocolVersion,
};
use super::util::produce;

pub type PacketProducer = fn(BytesMut, ProtocolVersion) -> anyhow::Result<PacketType<'static>>;

enum Mapping {
    Single(u8),
    List(Vec<(u8, ProtocolVersion)>),
}

enum Id {
    Serverbound(Mapping),
    Clientbound(Mapping),
    Both(Mapping, Mapping),
}

pub static HANDSHAKE_REG: sync::Lazy<StateRegistry> = sync::Lazy::new(|| {
    let mut registry = StateRegistry::new();
    registry.insert::<Handshake>(produce!(Handshake), Id::Serverbound(Mapping::Single(0x00)));
    registry
});

pub static STATUS_REG: sync::Lazy<StateRegistry> = sync::Lazy::new(|| {
    let mut registry = StateRegistry::new();
    registry.insert::<StatusRequest>(|_, _| Ok(PacketType::StatusRequest), Id::Serverbound(Mapping::Single(0x00)));
    registry.insert::<StatusResponse>(produce!(StatusResponse), Id::Clientbound(Mapping::Single(0x00)));
    registry.insert::<Ping>(produce!(Ping), Id::Both(Mapping::Single(0x01), Mapping::Single(0x01)));
    registry
});

pub static LOGIN_REG: sync::Lazy<StateRegistry> = sync::Lazy::new(|| {
    let mut registry = StateRegistry::new();
    registry.insert::<Disconnect>(produce!(Disconnect), Id::Clientbound(Mapping::Single(0x00)));
    registry.insert::<LoginStart>(produce!(LoginStart), Id::Serverbound(Mapping::Single(0x00)));
    registry.insert::<EncryptionRequest>(produce!(EncryptionRequest), Id::Clientbound(Mapping::Single(0x01)));
    registry.insert::<EncryptionResponse>(produce!(EncryptionResponse), Id::Serverbound(Mapping::Single(0x01)));
    registry.insert::<SetCompression>(produce!(SetCompression), Id::Clientbound(Mapping::Single(0x03)));
    registry.insert::<LoginSuccess>(produce!(LoginSuccess), Id::Clientbound(Mapping::Single(0x02)));
    registry
});

pub static PLAY_REG: sync::Lazy<StateRegistry> = sync::Lazy::new(|| {
    let mut registry = StateRegistry::new();
    registry.insert::<Disconnect>(produce!(Disconnect), Id::Clientbound(Mapping::List(vec![(0x1a, ProtocolVersion::V1_19_4), (0x17, ProtocolVersion::V1_19_3)])));
    registry.insert::<PluginMessage>(produce!(PluginMessage), Id::Clientbound(Mapping::Single(0x17)));
    registry.insert::<JoinGame>(produce!(JoinGame), Id::Clientbound(Mapping::Single(0x28)));
    registry.insert::<Respawn>(produce!(Respawn), Id::Clientbound(Mapping::Single(0x41)));
    registry
});

pub struct StateRegistry {
    protocols: HashMap<ProtocolVersion, PacketRegistry>,
}

impl StateRegistry {
    pub fn new() -> Self {
        Self {
            protocols: ProtocolVersion::iter().map(|x| (x, PacketRegistry::new())).collect(),
        }
    }

    pub fn get_registry(&self, direction: &Direction, protocol: &ProtocolVersion) -> (&ProtocolRegistry, &ProtocolRegistry) {
        let registry = self.protocols.get(protocol).unwrap();
        match direction {
            Direction::Clientbound => (&registry.clientbound, &registry.serverbound),
            Direction::Serverbound => (&registry.serverbound, &registry.clientbound),
        }
    }

    fn insert_mapping<T: Packet + 'static>(&mut self, producer: PacketProducer, mapping: Mapping, direction: Direction) {
        match mapping {
            Mapping::Single(id) => {
                for packet_registry in self.protocols.values_mut() {
                    match direction {
                        Direction::Clientbound => &mut packet_registry.clientbound,
                        Direction::Serverbound => &mut packet_registry.serverbound,
                    }
                    .insert::<T>(producer, id)
                }
            }
            Mapping::List(mut list) => {
                list.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

                for (index, (id, first_version)) in list.iter().enumerate() {
                    let next_version = list.get(index + 1).get_or_insert(&(0x00, ProtocolVersion::V1_19_4)).1;
                    let is_last = list.len() - 1 == index;

                    for (version, packet_registry) in &mut self.protocols {
                        if *version >= *first_version && (*version < next_version || is_last) {
                            match direction {
                                Direction::Clientbound => &mut packet_registry.clientbound,
                                Direction::Serverbound => &mut packet_registry.serverbound,
                            }
                            .insert::<T>(producer, *id);
                        }
                    }
                }
            }
        };
    }

    fn insert<T: Packet + 'static>(&mut self, packet_producer: PacketProducer, id: Id) {
        match id {
            Id::Serverbound(mapping) => self.insert_mapping::<T>(packet_producer, mapping, Direction::Serverbound),
            Id::Clientbound(mapping) => self.insert_mapping::<T>(packet_producer, mapping, Direction::Clientbound),
            Id::Both(server, client) => {
                self.insert_mapping::<T>(packet_producer, server, Direction::Serverbound);
                self.insert_mapping::<T>(packet_producer, client, Direction::Clientbound);
            }
        }
    }
}

struct PacketRegistry {
    pub serverbound: ProtocolRegistry,
    pub clientbound: ProtocolRegistry,
}
impl PacketRegistry {
    pub fn new() -> Self {
        Self {
            serverbound: ProtocolRegistry::new(),
            clientbound: ProtocolRegistry::new(),
        }
    }
}

pub struct ProtocolRegistry {
    packet_to_id: HashMap<TypeId, u8>,
    id_to_packeta: [Option<PacketProducer>; 128],
}

impl ProtocolRegistry {
    pub fn new() -> Self {
        Self {
            packet_to_id: HashMap::new(),
            id_to_packeta: [None; 128]
        }
    }

    fn insert<T: Packet + 'static>(&mut self, producer: PacketProducer, id: u8) {
        self.insert_packet_to_id::<T>(id);
        self.insert_id_to_packet(producer, id);
    }

    fn insert_packet_to_id<T: Packet + 'static>(&mut self, id: u8) {
        self.packet_to_id.insert(TypeId::of::<T>(), id);
    }

    fn insert_id_to_packet(&mut self, producer: PacketProducer, id: u8) {
        self.id_to_packeta[id as usize] = Some(producer);
    }

    pub fn get_packet(&self, id: u8) -> &Option<PacketProducer> {
        match self.id_to_packeta.get(id as usize) {
            Some(option) => option,
            None => &None
        }
    }

    pub fn get_id<T: Packet + 'static>(&self) -> anyhow::Result<&u8> {
        self.packet_to_id.get(&TypeId::of::<T>()).ok_or(anyhow!("Packet does not exist in this state or version"))
    }
}
