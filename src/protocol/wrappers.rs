use std::net::SocketAddr;

use uuid::Uuid;

use crate::component::Component;

use super::{codec::connection::Connection, packet::login::Disconnect};

pub struct Client {
    pub conn: Connection,
}

impl Client {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }

    pub async fn disconnect(mut self, reason: Component) -> anyhow::Result<()> {
        self.conn.write_packet(Disconnect { reason }).await?;
        self.conn.shutdown().await
    }
}

pub struct Server {
    pub address: SocketAddr,
    pub conn: Connection,
}

impl Server {
    pub fn new(conn: Connection, address: SocketAddr) -> Self {
        Self { address, conn }
    }
}

pub struct ConnectionInfo {
    pub username: String,
    pub uuid: Uuid,
    pub server: SocketAddr,
    pub boss_bars: Vec<Uuid>
}

impl ConnectionInfo {
    pub fn new(username: String, uuid: Uuid, server: SocketAddr) -> Self {
        Self { username, uuid, server, boss_bars: Vec::new() }
    }
}
