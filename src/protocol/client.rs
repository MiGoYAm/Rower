use crate::component::Component;

use super::{codec::connection::Connection, packet::login::Disconnect};

pub struct Client {
    connection: Connection
}

impl Client {
    pub async fn disconnect(&mut self, reason: Component) -> anyhow::Result<()> {
        self.connection.write_packet(Disconnect { reason }).await?;
        self.connection.shutdown().await
    }
}