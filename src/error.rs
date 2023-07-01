use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub struct PacketError {
    pub provided: i32,
    pub got: i32,
}

impl Display for PacketError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Invalid provided packet. Packet id: Provided: {}, Got: {}",
            self.provided, self.got
        )
    }
}

impl Error for PacketError {}

#[derive(Debug)]
pub struct ReadError;

impl Display for ReadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "")
    }
}

impl Error for ReadError {}
