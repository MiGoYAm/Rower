use anyhow::{anyhow, bail, Result};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use uuid::Uuid;

use crate::{
    component::Component,
    protocol::{
        buffer::{BufExt, BufMutExt},
        nbt::Compound,
        util::{get_array, put_array},
        Direction, ProtocolVersion, State,
    },
};

use super::{login::Disconnect, Packet, Packets, RawPacket};

pub struct PluginMessage {
    pub channel: String,
    pub data: Bytes,
}

impl Packet for PluginMessage {
    fn from_bytes(buf: &mut impl Buf, _: ProtocolVersion) -> Result<Self> {
        Ok(Self {
            channel: buf.get_identifier()?,
            data: buf.rest(),
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_string(&self.channel);
        buf.put_slice(&self.data);
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
    fn from_bytes(buf: &mut impl Buf, _: ProtocolVersion) -> Result<Self> {
        Ok(Self {
            entity_id: buf.get_i32(),
            is_hardcore: buf.get_bool()?,
            gamemode: buf.get_u8(),
            previous_gamemode: buf.get_u8(),
            dimensions_names: get_array(buf, |b| b.get_identifier())?,
            registry: Compound::read(buf)?,
            dimension_type: buf.get_identifier()?,
            dimension_name: buf.get_identifier()?,
            hashed_seed: buf.get_i64(),
            max_players: buf.get_varint()?,
            view_distance: buf.get_varint()?,
            simulation_distance: buf.get_varint()?,
            reduced_debug_info: buf.get_bool()?,
            respawn_screen: buf.get_bool()?,
            is_debug: buf.get_bool()?,
            is_flat: buf.get_bool()?,
            last_death: Death::get(buf)?,
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_i32(self.entity_id);
        buf.put_bool(self.is_hardcore);
        buf.put_u8(self.gamemode);
        buf.put_u8(self.previous_gamemode);
        buf.put_varint(self.dimensions_names.len() as i32);
        for dimension_name in self.dimensions_names {
            buf.put_string(&dimension_name);
        }
        self.registry.write(buf);
        buf.put_string(&self.dimension_type);
        buf.put_string(&self.dimension_name);
        buf.put_i64(self.hashed_seed);
        buf.put_varint(self.max_players);
        buf.put_varint(self.view_distance);
        buf.put_varint(self.simulation_distance);
        buf.put_bool(self.reduced_debug_info);
        buf.put_bool(self.respawn_screen);
        buf.put_bool(self.is_debug);
        buf.put_bool(self.is_flat);
        put_death(buf, self.last_death);
    }
}

#[derive(Clone)]
pub struct Death {
    pub dimension_name: String,
    pub position: i64,
}

impl Death {
    pub fn get(buf: &mut impl Buf) -> Result<Option<Self>> {
        buf.get_option(Self::get1)
    }

    pub fn get1(buf: &mut impl Buf) -> Result<Self> {
        Ok(Death {
            dimension_name: buf.get_identifier()?,
            position: buf.get_i64(),
        })
    }
}

fn put_death(buf: &mut BytesMut, death: Option<Death>) {
    if let Some(death) = death {
        buf.put_bool(true);
        buf.put_string(&death.dimension_name);
        buf.put_i64(death.position);
    } else {
        buf.put_bool(false);
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
    fn from_bytes(_: &mut impl Buf, _: ProtocolVersion) -> Result<Self> {
        unimplemented!()
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_string(&self.dimension_type);
        buf.put_string(&self.dimension_name);
        buf.put_i64(self.hashed_seed);
        buf.put_u8(self.gamemode);
        buf.put_u8(self.previous_gamemode);
        buf.put_bool(self.is_debug);
        buf.put_bool(self.is_flat);
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
            last_death: packet.last_death.clone(),
        }
    }
}

pub struct BossBar {
    pub uuid: Uuid,
    pub action: BossBarAction,
}

pub enum BossBarAction {
    Add {
        title: Component,
        health: f32,
        color: BossBarColor,
        division: BossBarDivision,
        flags: u8,
    },
    Remove,
    UpdateHealth(f32),
    UpdateTitle(Component),
    UpdateStyle(BossBarColor, BossBarDivision),
    UpdateFlags(u8),
}

pub enum BossBarColor {
    Pink,
    Blue,
    Red,
    Green,
    Yellow,
    Purple,
    White,
}

impl TryFrom<u8> for BossBarColor {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Pink),
            1 => Ok(Self::Blue),
            2 => Ok(Self::Red),
            3 => Ok(Self::Green),
            4 => Ok(Self::Yellow),
            5 => Ok(Self::Purple),
            6 => Ok(Self::White),
            value => Err(anyhow!("conversion from byte {} to bossbar color", value)),
        }
    }
}

pub enum BossBarDivision {
    None,
    SixNotches,
    TenNotches,
    TwelveNotches,
    TwentyNotches,
}

impl TryFrom<u8> for BossBarDivision {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::SixNotches),
            2 => Ok(Self::TenNotches),
            3 => Ok(Self::TwelveNotches),
            4 => Ok(Self::TwentyNotches),
            value => Err(anyhow!(
                "conversion from byte {} to bossbar division",
                value
            )),
        }
    }
}

impl Packet for BossBar {
    fn from_bytes(buf: &mut impl Buf, _: ProtocolVersion) -> Result<Self> {
        Ok(Self {
            uuid: buf.get_uuid(),
            action: match buf.get_u8() {
                0 => BossBarAction::Add {
                    title: buf.get_component()?,
                    health: buf.get_f32(),
                    color: buf.get_u8().try_into()?,
                    division: buf.get_u8().try_into()?,
                    flags: buf.get_u8(),
                },
                1 => BossBarAction::Remove,
                2 => BossBarAction::UpdateHealth(buf.get_f32()),
                3 => BossBarAction::UpdateTitle(buf.get_component()?),
                4 => BossBarAction::UpdateStyle(buf.get_u8().try_into()?, buf.get_u8().try_into()?),
                5 => BossBarAction::UpdateFlags(buf.get_u8()),
                value => bail!("bossbar decoding byte {}", value),
            },
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_uuid(self.uuid);

        match self.action {
            BossBarAction::Add {
                title,
                health,
                color,
                division,
                flags,
            } => {
                buf.put_u8(0);
                buf.put_component(&title).unwrap();
                buf.put_f32(health);
                buf.put_u8(color as u8);
                buf.put_u8(division as u8);
                buf.put_u8(flags);
            }
            BossBarAction::Remove => buf.put_u8(1),
            BossBarAction::UpdateHealth(health) => {
                buf.put_u8(2);
                buf.put_f32(health)
            }
            BossBarAction::UpdateTitle(title) => {
                buf.put_u8(3);
                buf.put_component(&title).unwrap();
            }
            BossBarAction::UpdateStyle(color, division) => {
                buf.put_u8(4);
                buf.put_u8(color as u8);
                buf.put_u8(division as u8);
            }
            BossBarAction::UpdateFlags(flags) => {
                buf.put_u8(5);
                buf.put_u8(flags);
            }
        }
    }
}

pub struct ChatCommand {
    pub command: String,
    pub timestamp: i64,
    pub salt: i64,
    pub arguments: Vec<(String, Bytes)>,
    pub message_count: i32,
    pub acknowledged: Bytes,
}

fn get_argument_signature(buf: &mut impl Buf) -> Result<(String, Bytes)> {
    Ok((buf.get_string(256)?, buf.get_bytes()?))
}

fn put_argument_signature(buf: &mut BytesMut, arg: &(String, Bytes)) {
    buf.put_string(&arg.0);
    buf.put_byte_array(&arg.1);
}

impl Packet for ChatCommand {
    fn from_bytes(buf: &mut impl Buf, _: ProtocolVersion) -> Result<Self> {
        Ok(Self {
            command: buf.get_string(256)?,
            timestamp: buf.get_i64(),
            salt: buf.get_i64(),
            arguments: get_array(buf, get_argument_signature)?,
            message_count: buf.get_varint()?,
            acknowledged: buf.rest(),
        })
    }

    fn put_buf(self, buf: &mut BytesMut, _: ProtocolVersion) {
        buf.put_string(&self.command);
        buf.put_i64(self.timestamp);
        buf.put_i64(self.salt);
        put_array(buf, self.arguments, put_argument_signature);
        buf.put_varint(self.message_count);
        buf.put_slice(&self.acknowledged);
        //buf.put_bitset(self.acknowledged);
    }
}

pub enum ClientPlay {
    Raw(RawPacket),
    ChatCommand(ChatCommand),
}

impl Packets for ClientPlay {
    fn decode(
        _direction: Direction,
        _state: State,
        _version: ProtocolVersion,
        packet: RawPacket,
    ) -> Result<Self> {
        Ok(Self::Raw(packet))
    }
}

pub enum ServerPlay {
    Raw(RawPacket),
    PluginMessage(PluginMessage),
    BossBar(BossBar),
    Disconnect(Disconnect),
}

impl Packets for ServerPlay {
    fn decode(
        _direction: Direction,
        _state: State,
        _version: ProtocolVersion,
        packet: RawPacket,
    ) -> Result<Self> {
        Ok(Self::Raw(packet))
    }
}
