use crate::component::Component;

use super::{codec::connection::Connection, packet::login::Disconnect};

pub struct Client {
    pub conn: Connection,
}

impl Client {
    pub fn from_conn(conn: Connection) -> Self {
        Self { conn }
    }

    pub async fn disconnect(mut self, reason: Component) -> anyhow::Result<()> {
        self.conn.write_packet(Disconnect { reason }).await?;
        self.conn.shutdown().await
    }
}
