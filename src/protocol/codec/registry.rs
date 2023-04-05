use std::{error::Error, any::TypeId, collections::HashMap, fmt};

use bytes::{BytesMut, Buf};
use once_cell::{sync};

use crate::protocol::{packet::{NextPacket, Packet, handshake::Handshake, status::{StatusResponse, StatusRequest, Ping}, Lazy, login::{Disconnect, LoginStart, LoginSuccess, SetCompression}, RawPacket}, Direction, V1_19_2};

type PacketProducer = fn(BytesMut, i32) -> NextPacket;

pub static HANDSHAKE_REGISTRY: sync::Lazy<StateRegistry> = sync::Lazy::new(|| {
    let mut registry = StateRegistry::new();
    registry.insert::<Handshake>(|b, v| NextPacket::Handshake(Lazy::new(b, v)), Id::Serverbound(0x00));
    registry
});

pub static STATUS_REGISTRY: sync::Lazy<StateRegistry> = sync::Lazy::new(|| {
    let mut registry = StateRegistry::new();
    registry.insert::<StatusRequest>(|b, v| NextPacket::Handshake(Lazy::new(b, v)), Id::Serverbound(0x00));
    registry.insert::<StatusResponse>(|b, v| NextPacket::Handshake(Lazy::new(b, v)), Id::Clientbound(0x00));
    registry.insert::<Ping>(|b, v| NextPacket::Ping(Lazy::new(b, v)), Id::Both(0x01, 0x01));
    registry
});

pub static LOGIN_REGISTRY: sync::Lazy<StateRegistry> = sync::Lazy::new(|| {
    let mut registry = StateRegistry::new();
    registry.insert::<Disconnect>(|b, v| NextPacket::Disconnect(Lazy::new(b, v)), Id::Clientbound(0x00));
    registry.insert::<LoginStart>(|b, v| NextPacket::Handshake(Lazy::new(b, v)), Id::Serverbound(0x00));
    registry.insert::<LoginSuccess>(|b, v| NextPacket::LoginSuccess(Lazy::new(b, v)), Id::Clientbound(0x02));
    registry.insert::<SetCompression>(|b, v| NextPacket::SetCompression(Lazy::new(b, v)), Id::Clientbound(0x03));
    registry
});

pub static PLAY_REGISTRY: sync::Lazy<StateRegistry> = sync::Lazy::new(|| {
    let mut registry = StateRegistry::new();
    registry
});

enum Id{
    Serverbound(u8),
    Clientbound(u8),
    Both(u8, u8)
}

pub struct StateRegistry {
    serverbound: Registry,
    clientbound: Registry
}

impl StateRegistry {
    pub fn new() -> Self {
        Self { serverbound: Registry::new(), clientbound: Registry::new() }
    }

    pub fn get_registry<'a>(&'a self, direction: &Direction) -> (&Registry, &Registry) {
        match direction {
            Direction::Clientbound => (&self.clientbound, &self.serverbound),
            Direction::Serverbound => (&self.serverbound, &self.clientbound)
        }
    }

    fn insert<T: Packet + 'static>(&mut self, p: PacketProducer, id: Id) {
        match id {
            Id::Serverbound(id) => self.serverbound.insert::<T>(p, id),
            Id::Clientbound(id) => self.clientbound.insert::<T>(p, id),
            Id::Both(server, client) => {
                self.serverbound.insert::<T>(p, server);
                self.clientbound.insert::<T>(p, client);
            },
        }
    }
}
pub struct Registry {
    packet_id: HashMap<TypeId, u8>,
    id_packet: HashMap<u8, PacketProducer>
}

impl Registry {
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

    pub fn decode(&self, mut buf: BytesMut, version: i32) -> NextPacket {
        let id = buf.get_u8();
        match self.id_packet.get(&id) {
            Some(v) => v(buf, version),
            None => NextPacket::RawPacket(RawPacket { id, data: buf }),
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