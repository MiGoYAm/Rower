use anyhow::{anyhow, ensure, Context, Result};
use std::{any::type_name, net::SocketAddr};

use bytes::Buf;
use futures::{SinkExt, StreamExt};
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};
use tokio_util::codec::{FramedRead, FramedWrite};

use crate::{
    component::Component,
    protocol::{
        packet::{login::Disconnect, IdPacket, Packet, PacketType, Packets, RawPacket},
        Direction, ProtocolVersion, State,
    },
};

use super::{
    decoder::MinecraftDecoder,
    encoder::MinecraftEncoder,
    registry::{get_protocol_registry, ProtocolRegistry},
};

pub const CLIENT: u8 = 0;
pub const SERVER: u8 = 1;
pub const CLIENT_SIDE: u8 = 2; // recv from server, send to client
pub const SERVER_SIDE: u8 = 3; // recv from client, send to server

struct Directions {
    pub recv: Direction,
    pub send: Direction,
}

const fn get_directions(id: u8) -> Directions {
    match id {
        CLIENT => Directions {
            recv: Direction::Clientbound,
            send: Direction::Serverbound,
        },
        SERVER => Directions {
            recv: Direction::Serverbound,
            send: Direction::Clientbound,
        },
        CLIENT_SIDE => Directions {
            recv: Direction::Clientbound,
            send: Direction::Clientbound,
        },
        SERVER_SIDE => Directions {
            recv: Direction::Serverbound,
            send: Direction::Serverbound,
        },
        _ => unreachable!(),
    }
}

pub type ClientConn<const S: u8> = Connection<{ CLIENT }, S>;
pub type ServerConn<const S: u8> = Connection<{ SERVER }, S>;
pub type ClientSideConn = Connection<{ CLIENT_SIDE }, { State::PLAY }>;
pub type ServerSideConn = Connection<{ SERVER_SIDE }, { State::PLAY }>;

pub struct Connection<const D: u8, const S: u8> {
    pub protocol: ProtocolVersion,

    framed_read: FramedRead<OwnedReadHalf, MinecraftDecoder>,
    framed_write: FramedWrite<OwnedWriteHalf, MinecraftEncoder>,
}

impl<const D: u8, const S: u8> Connection<D, S> {
    const DIRECTIONS: Directions = get_directions(D);
    const STATE: State = State::from_id(S);

    fn create(stream: TcpStream, protocol: ProtocolVersion) -> Self {
        let (reader, writer) = stream.into_split();

        Self {
            protocol,
            framed_read: FramedRead::new(reader, MinecraftDecoder::new()),
            framed_write: FramedWrite::new(writer, MinecraftEncoder::new()),
        }
    }

    pub fn new(stream: TcpStream) -> Self {
        Self::create(stream, ProtocolVersion::Unknown)
    }

    pub async fn connect_to(addr: SocketAddr, version: ProtocolVersion) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        stream.set_nodelay(true)?;
        Ok(Self::create(stream, version))
    }

    pub fn upgrade<const U: u8>(self) -> Connection<D, U> {
        Connection {
            protocol: self.protocol,
            framed_read: self.framed_read,
            framed_write: self.framed_write,
        }
    }

    fn receive_registry(&self) -> &'static ProtocolRegistry {
        get_protocol_registry(Self::DIRECTIONS.recv, Self::STATE, self.protocol).0
    }

    fn send_registry(&self) -> &'static ProtocolRegistry {
        get_protocol_registry(Self::DIRECTIONS.recv, Self::STATE, self.protocol).1
    }

    pub async fn auto_read(&mut self) -> Result<PacketType> {
        let mut packet = self.recv_raw_packet().await?;

        if let Some(producer) = self.receive_registry().get_packet(packet.id()) {
            let mut data = packet.data();
            let result = producer(&mut data, self.protocol)?;
            ensure!(data.is_empty(), "Packet was not been fully read");
            Ok(result)
        } else {
            Ok(PacketType::Raw(packet))
        }
    }

    pub async fn recv_packet<T: IdPacket + 'static>(&mut self) -> Result<T> {
        let id = self.expected_id::<T>(Self::DIRECTIONS.recv)?;
        self.deserizlize_packet(id).await
    }

    pub async fn recv_packet_dyn<T: Packet + 'static>(&mut self) -> Result<T> {
        let id = self.receive_registry().get_id::<T>()?;
        self.deserizlize_packet(*id).await
    }

    pub async fn recv_packets<T: Packets + 'static>(&mut self) -> Result<T> {
        let packet = self.recv_raw_packet().await?;
        T::decode(Self::DIRECTIONS.recv, Self::STATE, self.protocol, packet)
    }

    async fn deserizlize_packet<T: Packet>(&mut self, expected_id: u8) -> Result<T> {
        let mut frame = self.recv_raw_packet().await?.buffer;
        let id = frame.get_u8();

        ensure!(
            expected_id == id,
            "Invalid provided packet. Packet id: Provided: {:#04X?}, Got: {:#04X?}",
            expected_id,
            id
        );

        let packet = T::from_bytes(&mut frame, self.protocol).context(type_name::<T>())?;
        ensure!(
            frame.is_empty(),
            "Packet was not been fully read. Packet: {:}",
            type_name::<T>()
        );
        Ok(packet)
    }

    pub async fn recv_raw_packet(&mut self) -> Result<RawPacket> {
        match self.framed_read.next().await {
            Some(result) => Ok(RawPacket { buffer: result? }),
            None => Err(anyhow!("Connection aborted")),
        }
    }

    pub async fn queue_raw_packet(&mut self, packet: RawPacket) -> Result<()> {
        self.framed_write.feed(packet).await
    }

    pub async fn send_packet<T: IdPacket + 'static>(&mut self, packet: T) -> Result<()> {
        let raw_packet = self.serialize_packet_const(packet)?;
        self.framed_write.send(raw_packet).await
    }

    pub async fn send_packet_dyn<T: Packet + 'static>(&mut self, packet: T) -> Result<()> {
        let id = self.send_registry().get_id::<T>()?;
        let raw_packet = self.serialize_packet(packet, *id)?;
        self.framed_write.send(raw_packet).await
    }

    pub async fn queue_packet<T: IdPacket + 'static>(&mut self, packet: T) -> Result<()> {
        let raw_packet = self.serialize_packet_const(packet)?;
        self.framed_write.feed(raw_packet).await
    }

    pub async fn queue_packet_dyn<T: Packet + 'static>(&mut self, packet: T) -> Result<()> {
        let id = self.send_registry().get_id::<T>()?;
        let raw_packet = self.serialize_packet(packet, *id)?;
        self.framed_write.feed(raw_packet).await
    }

    fn expected_id<T: IdPacket + 'static>(&self, direction: Direction) -> Result<u8> {
        T::id(direction, Self::STATE, self.protocol).ok_or(
            anyhow!("Packet does not exist in this state or version").context(type_name::<T>()),
        )
    }

    fn serialize_packet_const<T: IdPacket + 'static>(&self, packet: T) -> Result<RawPacket> {
        let id = self.expected_id::<T>(Self::DIRECTIONS.send)?;
        self.serialize_packet(packet, id)
    }

    fn serialize_packet<T: Packet + 'static>(&self, packet: T, id: u8) -> Result<RawPacket> {
        let mut raw_packet = RawPacket::new();
        raw_packet.set_id(id);

        let mut data = raw_packet.data();
        packet.put_buf(&mut data, self.protocol);
        raw_packet.buffer.unsplit(data);

        Ok(raw_packet)
    }

    pub async fn shutdown(&mut self) -> Result<()> {
        self.framed_write.close().await
    }
}

impl<const S: u8> ClientConn<S> {
    pub async fn disconnect(mut self, reason: Component) -> Result<()> {
        self.send_packet_dyn(Disconnect { reason }).await?;
        self.shutdown().await
    }

    pub fn mix(self, server: ServerConn<{ State::PLAY }>) -> (ClientSideConn, ServerSideConn) {
        (
            ClientSideConn {
                protocol: self.protocol,
                framed_read: self.framed_read,
                framed_write: server.framed_write,
            },
            ServerSideConn {
                protocol: server.protocol,
                framed_read: server.framed_read,
                framed_write: self.framed_write,
            },
        )
    }
}

impl<const D: u8> Connection<D, { State::LOGIN }> {
    pub fn enable_compression(&mut self, threshold: u32) {
        self.framed_read.decoder_mut().enable_compression();
        self.framed_write
            .encoder_mut()
            .enable_compression(threshold);
    }

    pub fn enable_encryption(&mut self, key: [u8; 16]) -> Result<()> {
        self.framed_write.encoder_mut().enable_encryption(key)
    }
}
