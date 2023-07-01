use std::{error::Error, any::TypeId, collections::HashMap, fmt};

use bytes::{BytesMut, Buf};
use once_cell::sync;
use strum::IntoEnumIterator;

use crate::protocol::{packet::{Packet, PacketType, RawPacket, handshake::Handshake, Lazy, status::{StatusRequest, StatusResponse, Ping}, login::{Disconnect, LoginStart, LoginSuccess, SetCompression, EncryptionRequest, EncryptionResponse}, play::PluginMessage}, ProtocolVersion, Direction};

pub type PacketProducer = fn(BytesMut, ProtocolVersion) -> PacketType<'static>;

pub static HANDSHAKE_REG: sync::Lazy<StateRegistry> = sync::Lazy::new(|| {
    let mut registry = StateRegistry::new();
    registry.insert::<Handshake>(
        |b, v| PacketType::Handshake(Lazy::new(b, v)), 
        Id::Serverbound(Mapping::Single(0x00))
    );
    registry
});

pub static STATUS_REG: sync::Lazy<StateRegistry> = sync::Lazy::new(|| {
    let mut registry = StateRegistry::new();
    registry.insert::<StatusRequest>(
        |_, _| PacketType::StatusRequest, 
        Id::Serverbound(Mapping::Single(0x00))
    );
    registry.insert::<StatusResponse>(
        |b, v| PacketType::StatusResponse(Lazy::new(b, v)), 
        Id::Clientbound(Mapping::Single(0x00))
    );
    registry.insert::<Ping>(
        |b, v| PacketType::Ping(Lazy::new(b, v)), 
        Id::Both(Mapping::Single(0x01), Mapping::Single(0x01))
    );
    registry
});

pub static LOGIN_REG: sync::Lazy<StateRegistry> = sync::Lazy::new(|| {
    let mut registry = StateRegistry::new();
    registry.insert::<Disconnect>(
        |b, v| PacketType::Disconnect(Lazy::new(b, v)), 
        Id::Clientbound(Mapping::Single(0x00))
    );
    registry.insert::<LoginStart>(
        |b, v| PacketType::LoginStart(Lazy::new(b, v)), 
        Id::Serverbound(Mapping::Single(0x00))
    );
    registry.insert::<EncryptionRequest>(
        |b, v| PacketType::EncryptionRequest(Lazy::new(b, v)), 
        Id::Clientbound(Mapping::Single(0x01))
    );
    registry.insert::<EncryptionResponse>(
        |b, v| PacketType::EncryptionResponse(Lazy::new(b, v)), 
        Id::Serverbound(Mapping::Single(0x01))
    );
    registry.insert::<SetCompression>(
        |b, v| PacketType::SetCompression(Lazy::new(b, v)), 
        Id::Clientbound(Mapping::Single(0x03))
    );
    registry.insert::<LoginSuccess>(
        |b, v| PacketType::LoginSuccess(Lazy::new(b, v)), 
        Id::Clientbound(Mapping::Single(0x02))
    );
    registry
});

pub static PLAY_REG: sync::Lazy<StateRegistry> = sync::Lazy::new(|| {
    let mut registry = StateRegistry::new();
    registry.insert::<Disconnect>(
        |b, v| PacketType::Disconnect(Lazy::new(b, v)), 
        Id::Clientbound(Mapping::List(vec![(0x1a, ProtocolVersion::V1_19_4), (0x17, ProtocolVersion::V1_19_3)]))
    );
    registry.insert::<PluginMessage>(
        |b, v| PacketType::PluginMessage(Lazy::new(b, v)), 
        Id::Clientbound(Mapping::Single(0x17))
    );
    registry
});

enum Mapping {
    Single(u8),
    List(Vec<(u8, ProtocolVersion)>),
}

enum Id {
    Serverbound(Mapping),
    Clientbound(Mapping),
    Both(Mapping, Mapping),
}

pub struct StateRegistry {
    protocols: HashMap<ProtocolVersion, PacketRegistry>
}

impl StateRegistry {
    pub fn new() -> Self {
        Self { 
            protocols: ProtocolVersion::iter().map(|x| (x, PacketRegistry::new())).collect()
        }
    }

    pub fn get_registry<'a>(&'a self, direction: &Direction, protocol: &ProtocolVersion) -> (&ProtocolRegistry, &ProtocolRegistry) {
        let registry = self.protocols.get(&protocol).unwrap();
        match direction {
            Direction::Clientbound => (&registry.clientbound, &registry.serverbound),
            Direction::Serverbound => (&registry.serverbound, &registry.clientbound)
        }
    }

    fn insert_mapping<T: Packet + 'static>(&mut self, p: PacketProducer, mapping: Mapping, direction: Direction) {
        match mapping {
            Mapping::Single(id) => {
                for (_, packet_registry) in &mut self.protocols {
                    match direction {
                        Direction::Clientbound => &mut packet_registry.clientbound,
                        Direction::Serverbound => &mut packet_registry.serverbound,
                    }.insert::<T>(p, id)
                };
            },
            Mapping::List(mut list) => {
                list.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

                for (index, (id, first_version)) in list.iter().enumerate() {
                    let next_version = list.get(index+1).get_or_insert(&(0x00, ProtocolVersion::V1_19_4)).1;
                    let is_last = list.len()-1 == index;

                    for (version, packet_registry) in &mut self.protocols {
                        if *version >= *first_version && (*version < next_version || is_last) {
                            match direction {
                                Direction::Clientbound => &mut packet_registry.clientbound,
                                Direction::Serverbound => &mut packet_registry.serverbound,
                            }.insert::<T>(p, id.clone());
                        }
                    };
                }
            },
        };
    }

    fn insert<T: Packet + 'static>(&mut self, packet_producer: PacketProducer, id: Id) {
        match id {
            Id::Serverbound(mapping) => self.insert_mapping::<T>(packet_producer, mapping, Direction::Serverbound),
            Id::Clientbound(mapping) => self.insert_mapping::<T>(packet_producer, mapping, Direction::Clientbound),
            Id::Both(server, client) => {
                self.insert_mapping::<T>(packet_producer, server, Direction::Serverbound);
                self.insert_mapping::<T>(packet_producer, client, Direction::Clientbound);
            },
        }
    }
}

struct PacketRegistry {
    pub serverbound: ProtocolRegistry,
    pub clientbound: ProtocolRegistry
}
impl PacketRegistry {
    pub fn new() -> Self {
        Self { serverbound: ProtocolRegistry::new(), clientbound: ProtocolRegistry::new() }
    }
}

pub struct ProtocolRegistry {
    packet_id: HashMap<TypeId, u8>,
    id_packet: HashMap<u8, PacketProducer>
}

impl ProtocolRegistry {
    pub fn new() -> Self {
        Self {
            packet_id: HashMap::new(),
            id_packet: HashMap::new()
        }
    }

    fn insert<T: Packet + 'static>(&mut self, p: PacketProducer, id: u8) {
        self.packet_id.insert(TypeId::of::<T>(), id);
        self.id_packet.insert(id, p);
    }

    pub fn decode(&self, mut buf: BytesMut, version: ProtocolVersion) -> PacketType {
        let id = buf.get_u8();
        match self.id_packet.get(&id) {
            Some(v) => v(buf, version),
            None => PacketType::Raw(RawPacket { id, data: buf }),
        }
    }

    pub fn get_packet(&self, id: u8) -> Result<&PacketProducer, Box<dyn Error>> {
        match self.id_packet.get(&id) {
            Some(v) => Ok(v),
            None => Err(PacketId(id).into()),
        }
    }

    pub fn get_id<T: Packet + 'static>(&self) -> Result<&u8, Box<dyn Error>> {
        match self.packet_id.get(&TypeId::of::<T>()) {
            Some(v) => Ok(v),
            None => Err(PacketErr.into()),
        }
    }
}

#[derive(Debug, Clone)]
struct PacketErr;

impl std::error::Error for PacketErr {}

impl fmt::Display for PacketErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "packet does not exist in this state or version")
    }
}

#[derive(Debug, Clone)]
struct PacketId(u8);

impl std::error::Error for PacketId {}

impl fmt::Display for PacketId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "packet with id {:02X?} does not exist in this state or version", self.0)
    }
}
