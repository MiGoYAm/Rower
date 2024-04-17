use std::{
    any::{type_name, TypeId},
    collections::HashMap,
    sync::LazyLock,
    vec,
};

use anyhow::{anyhow, Result};
use bytes::BytesMut;
use strum::IntoEnumIterator;

use super::util::produce;
use crate::protocol::{
    packet::{
        login::{Disconnect, EncryptionResponse, LoginStart},
        play::{BossBar, ChatCommand, JoinGame, PluginMessage, Respawn},
        Packet, PacketType,
    },
    Direction, ProtocolVersion, State,
};

pub type PacketProducer = fn(&mut BytesMut, ProtocolVersion) -> Result<PacketType>;

pub fn get_protocol_registry(
    direction: Direction,
    state: State,
    version: ProtocolVersion,
) -> (&'static ProtocolRegistry, &'static ProtocolRegistry) {
    PLAY_REG.get_registries(direction, version)
}

enum Mapping {
    Single(u8),
    List(Vec<(u8, ProtocolVersion)>),
}

enum Id {
    Serverbound(Mapping),
    Clientbound(Mapping),
    Both(Mapping, Mapping),
}

pub static LOGIN_REG: LazyLock<StateRegistry> = LazyLock::new(|| {
    let mut reg = StateRegistry::new();
    reg.insert::<Disconnect>(produce!(Disconnect), Id::Clientbound(Mapping::Single(0x00)));
    reg.insert::<LoginStart>(None, Id::Serverbound(Mapping::Single(0x00)));
    reg.insert::<EncryptionResponse>(None, Id::Serverbound(Mapping::Single(0x01)));
    reg
});

pub static PLAY_REG: LazyLock<StateRegistry> = LazyLock::new(|| {
    let mut reg = StateRegistry::new();
    reg.insert::<Disconnect>(
        produce!(Disconnect),
        Id::Clientbound(Mapping::List(vec![
            (0x1a, ProtocolVersion::V1_19_4),
            (0x17, ProtocolVersion::V1_19_3),
        ])),
    );
    reg.insert::<PluginMessage>(
        produce!(PluginMessage),
        Id::Clientbound(Mapping::Single(0x17)),
    );
    reg.insert::<JoinGame>(None, Id::Clientbound(Mapping::Single(0x28)));
    reg.insert::<Respawn>(None, Id::Clientbound(Mapping::Single(0x41)));
    reg.insert::<BossBar>(produce!(BossBar), Id::Clientbound(Mapping::Single(0x0b)));
    reg.insert::<ChatCommand>(
        produce!(ChatCommand),
        Id::Serverbound(Mapping::Single(0x04)),
    );
    reg
});

pub struct StateRegistry {
    protocols: Vec<PacketRegistry>,
}

impl StateRegistry {
    pub fn new() -> Self {
        Self {
            protocols: vec![PacketRegistry::new(); ProtocolVersion::iter().count()],
        }
    }

    pub fn get_registries(
        &self,
        direction: Direction,
        protocol: ProtocolVersion,
    ) -> (&ProtocolRegistry, &ProtocolRegistry) {
        self.protocols
            .get(protocol as usize)
            .unwrap()
            .get_registry(direction)
    }

    pub fn get_registry(&self, direction: Direction, protocol: ProtocolVersion) -> &ProtocolRegistry {
        let registries = self.protocols.get(protocol as usize).unwrap();
        match direction {
            Direction::Clientbound => &registries.clientbound,
            Direction::Serverbound => &registries.serverbound,
        }
    }

    fn some<T: Packet + 'static>(
        registry: &mut PacketRegistry,
        direction: Direction,
        id: u8,
        producer: Option<PacketProducer>,
    ) {
        let registry = match direction {
            Direction::Clientbound => &mut registry.clientbound,
            Direction::Serverbound => &mut registry.serverbound,
        };
        match producer {
            Some(producer) => registry.insert::<T>(producer, id),
            None => registry.insert_packet_to_id::<T>(id),
        }
    }

    fn insert_mapping<T: Packet + 'static>(
        &mut self,
        producer: Option<PacketProducer>,
        mapping: Mapping,
        direction: Direction,
    ) {
        match mapping {
            Mapping::Single(id) => {
                for packet_registry in &mut self.protocols {
                    Self::some::<T>(packet_registry, direction, id, producer);
                }
            }
            Mapping::List(mut list) => {
                list.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

                for (index, (id, first_version)) in
                    list.iter().map(|(i, v)| (i, *v as usize)).enumerate()
                {
                    let next_version = *list
                        .get(index + 1)
                        .map(|e| e.1 as usize)
                        .get_or_insert(ProtocolVersion::V1_19_4 as usize);
                    let is_last = list.len() - 1 == index;

                    for (version, packet_registry) in self.protocols.iter_mut().enumerate() {
                        if version >= first_version && (version < next_version || is_last) {
                            Self::some::<T>(packet_registry, direction, *id, producer);
                        }
                    }
                }
            }
        };
    }

    fn insert<T: Packet + 'static>(&mut self, packet_producer: Option<PacketProducer>, id: Id) {
        match id {
            Id::Serverbound(mapping) => {
                self.insert_mapping::<T>(packet_producer, mapping, Direction::Serverbound)
            }
            Id::Clientbound(mapping) => {
                self.insert_mapping::<T>(packet_producer, mapping, Direction::Clientbound)
            }
            Id::Both(server, client) => {
                self.insert_mapping::<T>(packet_producer, server, Direction::Serverbound);
                self.insert_mapping::<T>(packet_producer, client, Direction::Clientbound);
            }
        }
    }
}

#[derive(Clone)]
pub struct PacketRegistry {
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

    pub const fn get_registry(
        &self,
        direction: Direction,
    ) -> (&ProtocolRegistry, &ProtocolRegistry) {
        match direction {
            Direction::Serverbound => (&self.clientbound, &self.serverbound),
            Direction::Clientbound => (&self.serverbound, &self.clientbound),
        }
    }
}

#[derive(Clone)]
pub struct ProtocolRegistry {
    packet_to_id: HashMap<TypeId, u8>,
    id_to_packeta: [Option<PacketProducer>; 128],
}

impl ProtocolRegistry {
    pub fn new() -> Self {
        Self {
            packet_to_id: HashMap::new(),
            id_to_packeta: [None; 128],
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

    pub fn get_packet(&self, id: u8) -> Option<&PacketProducer> {
        match self.id_to_packeta.get(id as usize) {
            Some(option) => option.as_ref(),
            None => None,
        }
    }

    pub fn get_id<T: Packet + 'static>(&self) -> Result<&u8> {
        self.packet_to_id.get(&TypeId::of::<T>()).ok_or(
            anyhow!("Packet does not exist in this state or version").context(type_name::<T>()),
        )
    }
}
