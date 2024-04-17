use std::net::SocketAddr;

use uuid::Uuid;

use crate::online::generate_offline_uuid;

use super::codec::connection::ServerConn;

pub struct Server<const S: u8> {
    pub address: SocketAddr,
    pub conn: ServerConn<S>
}

impl<const S: u8> Server<S> {
    pub fn new(conn: ServerConn<S>, address: SocketAddr) -> Self {
        Self { address, conn }
    }
}

pub struct ConnectionInfo {
    pub username: String,
    pub uuid: Uuid,
    pub boss_bars: Vec<Uuid>
}

impl ConnectionInfo {
    pub fn new(username: String, uuid: Option<Uuid>) -> Self {
        let uuid = uuid.unwrap_or_else(|| generate_offline_uuid(&username));
        Self { username, uuid, boss_bars: Vec::new() }
    }
}
