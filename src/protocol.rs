#![allow(dead_code)]
use md5::{Digest, Md5};
use strum::EnumIter;
use uuid::Uuid;

pub mod codec;
pub mod packet;
pub mod util;
pub mod wrappers;
pub mod nbt;
pub mod buffer;

pub enum State {
    Handshake,
    Status,
    Login,
    Play
}

#[derive(Clone, Copy)]
pub enum Direction {
    Clientbound,
    Serverbound,
}

#[repr(usize)]
#[derive(PartialOrd, Ord, PartialEq, Eq, Copy, Clone, EnumIter)]
pub enum ProtocolVersion {
    V1_19_4,
    V1_19_3,
    V1_19_2,
    V1_19,
    V1_18_2,
    V1_18,
    V1_17_1,
    V1_17,
    V1_16_4,
    V1_16_3,
    V1_16_2,
    V1_16_1,
    V1_16,
    V1_15_2,
    V1_15_1,
    V1_15,
    V1_14_4,
    V1_14_3,
    V1_14_2,
    V1_14_1,
    V1_14,
    V1_13_2,
    V1_13_1,
    V1_13,
    V1_12_2,
    V1_12_1,
    V1_12,
    V1_11_1,
    V1_11,
    V1_10,
    V1_9_4,
    V1_9_2,
    V1_9_1,
    V1_9,
    V1_8,
    V1_7_6,
    V1_7_2,
    Unknown,
}

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

impl std::convert::From<i32> for ProtocolVersion {
    fn from(value: i32) -> Self {
        match value {
            V1_19_4 => ProtocolVersion::V1_19_4,
            V1_19_3 => ProtocolVersion::V1_19_3,
            V1_19_2 => ProtocolVersion::V1_19_2,
            V1_19 => ProtocolVersion::V1_19,
            V1_18_2 => ProtocolVersion::V1_18_2,
            V1_18 => ProtocolVersion::V1_18,
            V1_17_1 => ProtocolVersion::V1_17_1,
            V1_17 => ProtocolVersion::V1_17,
            V1_16_4 => ProtocolVersion::V1_16_4,
            V1_16_3 => ProtocolVersion::V1_16_3,
            V1_16_2 => ProtocolVersion::V1_16_2,
            V1_16_1 => ProtocolVersion::V1_16_1,
            V1_16 => ProtocolVersion::V1_16,
            V1_15_2 => ProtocolVersion::V1_15_2,
            V1_15_1 => ProtocolVersion::V1_15_1,
            V1_15 => ProtocolVersion::V1_15,
            V1_14_4 => ProtocolVersion::V1_14_4,
            V1_14_3 => ProtocolVersion::V1_14_3,
            V1_14_2 => ProtocolVersion::V1_14_2,
            V1_14_1 => ProtocolVersion::V1_14_1,
            V1_14 => ProtocolVersion::V1_14,
            V1_13_2 => ProtocolVersion::V1_13_2,
            V1_13_1 => ProtocolVersion::V1_13_1,
            V1_13 => ProtocolVersion::V1_13,
            V1_12_2 => ProtocolVersion::V1_12_2,
            V1_12_1 => ProtocolVersion::V1_12_1,
            V1_12 => ProtocolVersion::V1_12,
            V1_11_1 => ProtocolVersion::V1_11_1,
            V1_11 => ProtocolVersion::V1_11,
            V1_10 => ProtocolVersion::V1_10,
            V1_9_4 => ProtocolVersion::V1_9_4,
            V1_9_2 => ProtocolVersion::V1_9_2,
            V1_9_1 => ProtocolVersion::V1_9_1,
            V1_9 => ProtocolVersion::V1_9,
            V1_8 => ProtocolVersion::V1_8,
            V1_7_6 => ProtocolVersion::V1_7_6,
            V1_7_2 => ProtocolVersion::V1_7_2,
            _ => ProtocolVersion::Unknown
        }
    }
}

impl std::convert::From<ProtocolVersion> for i32 {
    fn from(val: ProtocolVersion) -> Self {
        match val {
            ProtocolVersion::V1_19_4 => V1_19_4,
            ProtocolVersion::V1_19_3 => V1_19_3,
            ProtocolVersion::V1_19_2 => V1_19_2,
            ProtocolVersion::V1_19 => V1_19,
            ProtocolVersion::V1_18_2 => V1_18_2,
            ProtocolVersion::V1_18 => V1_18,
            ProtocolVersion::V1_17_1 => V1_17_1,
            ProtocolVersion::V1_17 => V1_17,
            ProtocolVersion::V1_16_4 => V1_16_4,
            ProtocolVersion::V1_16_3 => V1_16_3,
            ProtocolVersion::V1_16_2 => V1_16_2,
            ProtocolVersion::V1_16_1 => V1_16_1,
            ProtocolVersion::V1_16 => V1_16,
            ProtocolVersion::V1_15_2 => V1_15_2,
            ProtocolVersion::V1_15_1 => V1_15_1,
            ProtocolVersion::V1_15 => V1_15,
            ProtocolVersion::V1_14_4 => V1_14_4,
            ProtocolVersion::V1_14_3 => V1_14_3,
            ProtocolVersion::V1_14_2 => V1_14_2,
            ProtocolVersion::V1_14_1 => V1_14_1,
            ProtocolVersion::V1_14 => V1_14,
            ProtocolVersion::V1_13_2 => V1_13_2,
            ProtocolVersion::V1_13_1 => V1_13_1,
            ProtocolVersion::V1_13 => V1_13,
            ProtocolVersion::V1_12_2 => V1_12_2,
            ProtocolVersion::V1_12_1 => V1_12_1,
            ProtocolVersion::V1_12 => V1_12,
            ProtocolVersion::V1_11_1 => V1_11_1,
            ProtocolVersion::V1_11 => V1_11,
            ProtocolVersion::V1_10 => V1_10,
            ProtocolVersion::V1_9_4 => V1_9_4,
            ProtocolVersion::V1_9_2 => V1_9_2,
            ProtocolVersion::V1_9_1 => V1_9_1,
            ProtocolVersion::V1_9 => V1_9,
            ProtocolVersion::V1_8 => V1_8,
            ProtocolVersion::V1_7_6 => V1_7_6,
            ProtocolVersion::V1_7_2 => V1_7_2,
            ProtocolVersion::Unknown => -1
            //ProtocolVersion::Unknown(version) => version,
        }
    }
}

pub fn generate_offline_uuid(username: &String) -> Uuid {
    let mut hasher = Md5::new_with_prefix(b"OfflinePlayer:");
    hasher.update(username.as_bytes());
    let hash = hasher.finalize();
    (ProtocolVersion::V1_7_2..ProtocolVersion::V1_19_4);

    uuid::Builder::from_md5_bytes(hash.into()).into_uuid()
}
