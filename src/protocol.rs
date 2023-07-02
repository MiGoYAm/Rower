#![allow(dead_code)]

use std::error::Error;

use md5::{Md5, Digest};
use strum::EnumIter;
use uuid::Uuid;

//pub mod connection;
pub mod packet;
pub mod util;
pub mod codec;

pub const HANDSHAKE: u8 = 0;
pub const STATUS: u8 = 1;
pub const LOGIN: u8 = 2;
pub const PLAY: u8 = 3;

pub enum Direction {
    Clientbound,
    Serverbound,
}

// 36
pub const V_UNKNOWN: i32 = -1;
pub const V1_19_4: i32 = 762;
pub const V1_19_3: i32 = 761;
pub const V1_19_2: i32 = 760;
pub const V1_19: i32 = 759;
pub const V1_18_2: i32 = 758;
pub const V1_18: i32 = 757;
pub const V1_17_1: i32 = 756;
pub const V1_17: i32 = 755;
pub const V1_16_4: i32 = 754;
pub const V1_16_3: i32 = 753;
pub const V1_16_2: i32 = 751;
pub const V1_16_1: i32 = 736;
pub const V1_16: i32 = 735;
pub const V1_15_2: i32 = 578;
pub const V1_15_1: i32 = 575;
pub const V1_15: i32 = 573;
pub const V1_14_4: i32 = 498;
pub const V1_14_3: i32 = 490;
pub const V1_14_2: i32 = 485;
pub const V1_14_1: i32 = 480;
pub const V1_14: i32 = 477;
pub const V1_13_2: i32 = 404;
pub const V1_13_1: i32 = 401;
pub const V1_13: i32 = 393;
pub const V1_12_2: i32 = 340;
pub const V1_12_1: i32 = 338;
pub const V1_12: i32 = 335;
pub const V1_11_1: i32 = 316;
pub const V1_11: i32 = 315;
pub const V1_10: i32 = 210;
pub const V1_9_4: i32 = 110;
pub const V1_9_2: i32 = 109;
pub const V1_9_1: i32 = 108;
pub const V1_9: i32 = 107;
pub const V1_8: i32 = 47;
pub const V1_7_6: i32 = 5;
pub const V1_7_2: i32 = 4;

#[repr(i32)]
#[derive(PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Hash, EnumIter)]
pub enum ProtocolVersion {
    Unknown = -1,
    V1_19_4 = 762,
    V1_19_3 = 761,
    V1_19_2 = 760,
    V1_19 = 759,
    V1_18_2 = 758,
    V1_18 = 757,
    V1_17_1 = 756,
    V1_17 = 755,
    V1_16_4 = 754,
    V1_16_3 = 753,
    V1_16_2 = 751,
    V1_16_1 = 736,
    V1_16 = 735,
    V1_15_2 = 578,
    V1_15_1 = 575,
    V1_15 = 573,
    V1_14_4 = 498,
    V1_14_3 = 490,
    V1_14_2 = 485,
    V1_14_1 = 480,
    V1_14 = 477,
    V1_13_2 = 404,
    V1_13_1 = 401,
    V1_13 = 393,
    V1_12_2 = 340,
    V1_12_1 = 338,
    V1_12 = 335,
    V1_11_1 = 316,
    V1_11 = 315,
    V1_10 = 210,
    V1_9_4 = 110,
    V1_9_2 = 109,
    V1_9_1 = 108,
    V1_9 = 107,
    V1_8 = 47,
    V1_7_6 = 5,
    V1_7_2 = 4,
}

impl std::convert::TryFrom<i32> for ProtocolVersion {
    type Error = Box<dyn Error>;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            762 => Ok(ProtocolVersion::V1_19_4),
            761 => Ok(ProtocolVersion::V1_19_3),
            760 => Ok(ProtocolVersion::V1_19_2),
            759 => Ok(ProtocolVersion::V1_19),
            758 => Ok(ProtocolVersion::V1_18_2),
            757 => Ok(ProtocolVersion::V1_18),
            756 => Ok(ProtocolVersion::V1_17_1),
            755 => Ok(ProtocolVersion::V1_17),
            754 => Ok(ProtocolVersion::V1_16_4),
            753 => Ok(ProtocolVersion::V1_16_3),
            751 => Ok(ProtocolVersion::V1_16_2),
            736 => Ok(ProtocolVersion::V1_16_1),
            735 => Ok(ProtocolVersion::V1_16),
            578 => Ok(ProtocolVersion::V1_15_2),
            575 => Ok(ProtocolVersion::V1_15_1),
            573 => Ok(ProtocolVersion::V1_15),
            498 => Ok(ProtocolVersion::V1_14_4),
            490 => Ok(ProtocolVersion::V1_14_3),
            485 => Ok(ProtocolVersion::V1_14_2),
            480 => Ok(ProtocolVersion::V1_14_1),
            477 => Ok(ProtocolVersion::V1_14),
            404 => Ok(ProtocolVersion::V1_13_2),
            401 => Ok(ProtocolVersion::V1_13_1),
            393 => Ok(ProtocolVersion::V1_13),
            340 => Ok(ProtocolVersion::V1_12_2),
            338 => Ok(ProtocolVersion::V1_12_1),
            335 => Ok(ProtocolVersion::V1_12),
            316 => Ok(ProtocolVersion::V1_11_1),
            315 => Ok(ProtocolVersion::V1_11),
            210 => Ok(ProtocolVersion::V1_10),
            110 => Ok(ProtocolVersion::V1_9_4),
            109 => Ok(ProtocolVersion::V1_9_2),
            108 => Ok(ProtocolVersion::V1_9_1),
            107 => Ok(ProtocolVersion::V1_9),
            47 => Ok(ProtocolVersion::V1_8),
            5 => Ok(ProtocolVersion::V1_7_6),
            4 => Ok(ProtocolVersion::V1_7_2),
            v @ _ => Err(format!("Could not convert u32({}) to ProtocolVersion", v).into()),
        }
    }
}


pub fn generate_offline_uuid(username: &String) -> Uuid {
    let mut hasher = Md5::new();
    hasher.update(("OfflinePlayer:".to_string() + username).as_bytes());
    let hash = hasher.finalize();

    uuid::Builder::from_md5_bytes(hash.into()).into_uuid()
}