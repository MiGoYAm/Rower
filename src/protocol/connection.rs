use std::error::Error;

use crate::protocol::util;
use crate::protocol::util::write_varint;
use bytes::{BytesMut, Buf};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufStream, self};
use tokio::net::TcpStream;

use super::packet::{RawPacket, Packet};

pub struct DeprectedConnection {
    stream: BufStream<TcpStream>,
    pub protocol_version: i32
}

impl DeprectedConnection {
    pub fn new(socket: TcpStream) -> Self {
        Self { stream: BufStream::new(socket), protocol_version: 0 }
    }

    pub async fn shutdown(&mut self) -> Result<(), io::Error> {
        self.stream.shutdown().await
    }

    pub async fn read_packet<T: Packet>(&mut self) -> Result<T, Box<dyn Error>> {
        let mut raw_packet = self.read_raw_packet().await?;

        if T::serverbound_id(self.protocol_version) != raw_packet.id {
            Err(format!("Invalid provided packet. Packet id: Provided: {}, Got: {}", T::serverbound_id(self.protocol_version), raw_packet.id))?;
        }

        T::from_bytes(&mut raw_packet.data, self.protocol_version)
    }

    pub async fn read_raw_packet(&mut self) -> Result<RawPacket, Box<dyn Error>> {
        let mut buf = self.decode_frame().await?;

        Ok(RawPacket {
            id: buf.get_u8(),
            data: buf
        })
    }

    async fn decode_frame(&mut self) -> Result<BytesMut, Box<dyn Error>> {
        let len = util::read_varint(&mut self.stream).await?;

        let mut buf = BytesMut::with_capacity(len as usize);
        self.stream.read_buf(&mut buf).await?;

        Ok(buf)
    }

    pub async fn write_packet<T: Packet>(&mut self, packet: T) -> Result<(), Box<dyn Error>> {
        self.write_packet_without_flush(packet).await?;

        self.stream.flush().await?;
        Ok(())
    }

    pub async fn write_packet_without_flush<T: Packet>(&mut self, packet: T) -> Result<(), Box<dyn Error>> {
        let mut buf = BytesMut::new();

        packet.put_buf(&mut buf, self.protocol_version);

        self.write_raw_packet(RawPacket {
            id: T::clientbound_id(self.protocol_version),
            data: buf
        }).await?;

        Ok(())
    }

    pub async fn write_raw_packet(&mut self, mut packet: RawPacket) -> Result<(), Box<dyn Error>> {
        write_varint(&mut self.stream, (1 + packet.data.len()) as i32).await?;
        self.stream.write_u8(packet.id).await?;
        self.stream.write_buf(&mut packet.data).await?;
        Ok(())
    }

}
