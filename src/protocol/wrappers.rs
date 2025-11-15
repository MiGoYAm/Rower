use uuid::Uuid;

use crate::online::generate_offline_uuid;

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
