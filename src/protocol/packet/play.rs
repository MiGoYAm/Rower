use bytes::{BytesMut, BufMut, Buf};

use crate::protocol::{
    util::{put_bool, put_varint, get_bool, get_array, get_identifier, get_varint, put_string},
    ProtocolVersion, nbt::Compound,
};

use super::Packet;

pub struct PluginMessage {
    pub channel: String,
    pub data: BytesMut,
}

impl Packet for PluginMessage {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> anyhow::Result<Self> {
        Ok(Self {
            channel: get_identifier(buf)?,
            data: buf.split(),
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        put_string(buf, &self.channel);
        buf.extend_from_slice(&self.data);
    }
}

pub struct JoinGame {
    pub entity_id: i32,
    pub is_hardcore: bool,
    pub gamemode: u8,
    pub previous_gamemode: u8,
    pub dimensions_names: Vec<String>,

    pub registry: Compound,

    pub dimension_type: String,
    pub dimension_name: String,
    pub hashed_seed: i64,
    pub max_players: i32,
    pub view_distance: i32,
    pub simulation_distance: i32,
    pub reduced_debug_info: bool,
    pub respawn_screen: bool,
    pub is_debug: bool,
    pub is_flat: bool,
    pub last_death: Option<Death>,
    //pub portal_cooldown: i32, // 1.20+
}

impl Packet for JoinGame {
    fn from_bytes(buf: &mut BytesMut, _: ProtocolVersion) -> anyhow::Result<Self> {
        Ok(Self {
            entity_id: buf.get_i32(),
            is_hardcore: get_bool(buf)?,
            gamemode: buf.get_u8(),
            previous_gamemode: buf.get_u8(),
            dimensions_names: get_array(buf, get_identifier)?,
            registry: Compound::read(buf)?,
            dimension_type: get_identifier(buf)?,
            dimension_name: get_identifier(buf)?,
            hashed_seed: buf.get_i64(),
            max_players: get_varint(buf)?,
            view_distance: get_varint(buf)?,
            simulation_distance: get_varint(buf)?,
            reduced_debug_info: get_bool(buf)?,
            respawn_screen: get_bool(buf)?,
            is_debug: get_bool(buf)?,
            is_flat: get_bool(buf)?,
            last_death: Death::get(buf)?,
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_i32(self.entity_id);
        put_bool(buf, self.is_hardcore);
        buf.put_u8(self.gamemode);
        buf.put_u8(self.previous_gamemode);
        put_varint(buf, self.dimensions_names.len() as u32);
        for dimension_name in self.dimensions_names {
            put_string(buf, &dimension_name);
        }
        self.registry.write(buf);
        put_string(buf, &self.dimension_type);
        put_string(buf, &self.dimension_name);
        buf.put_i64(self.hashed_seed);
        put_varint(buf, self.max_players as u32);
        put_varint(buf, self.view_distance as u32);
        put_varint(buf, self.simulation_distance as u32);
        put_bool(buf, self.reduced_debug_info);
        put_bool(buf, self.respawn_screen);
        put_bool(buf, self.is_debug);
        put_bool(buf, self.is_flat);
        put_death(buf, self.last_death);
    }
}

#[derive(Clone)]
pub struct Death {
    pub dimension_name: String,
    pub position: i64,
}

impl Death {
    pub fn get(buf: &mut BytesMut) -> anyhow::Result<Option<Self>> {
        if get_bool(buf)? {
            Ok(Some(Death {
                dimension_name: get_identifier(buf)?,
                position: buf.get_i64(),
            }))
        } else {
            Ok(None)
        }
    }
}

fn put_death(buf: &mut BytesMut, death: Option<Death>) {
    if let Some(death) = death {
        put_bool(buf, true);
        put_string(buf, &death.dimension_name);
        buf.put_i64(death.position);
    } else {
        put_bool(buf, false);
    }
}

pub struct Respawn {
    pub dimension_type: String,
    pub dimension_name: String,
    pub hashed_seed: i64,
    pub gamemode: u8,
    pub previous_gamemode: u8,
    pub is_debug: bool,
    pub is_flat: bool,
    pub data_kept: u8,
    pub last_death: Option<Death>,
    //pub portal_cooldown: i32, // 1.20+
}

impl Packet for Respawn {
    fn from_bytes(_: &mut BytesMut, _: ProtocolVersion) -> anyhow::Result<Self> {
        unreachable!()
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        put_string(buf, &self.dimension_type);
        put_string(buf, &self.dimension_name);
        buf.put_i64(self.hashed_seed);
        buf.put_u8(self.gamemode);
        buf.put_u8(self.previous_gamemode);
        put_bool(buf, self.is_debug);
        put_bool(buf, self.is_flat);
        buf.put_u8(self.data_kept);
        put_death(buf, self.last_death);
    }
}

impl Respawn {
    pub fn from_joingame(packet: &JoinGame) -> Self {
        Self { 
            dimension_type: packet.dimension_type.clone(),
            dimension_name: packet.dimension_name.clone(),
            hashed_seed: packet.hashed_seed,
            gamemode: packet.gamemode,
            previous_gamemode: packet.previous_gamemode,
            is_debug: packet.is_debug,
            is_flat: packet.is_flat,
            data_kept: 0,
            last_death: packet.last_death.clone()
        }
    }
}
