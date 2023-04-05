#![allow(dead_code)]

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
