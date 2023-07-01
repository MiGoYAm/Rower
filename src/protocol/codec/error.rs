use std::fmt;

#[derive(Debug, Clone)]
pub struct VarintTooBig;

impl std::error::Error for VarintTooBig {}

impl fmt::Display for VarintTooBig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "readed varint was too big")
    }
}

#[derive(Debug, Clone)]
pub struct FrameToobig;

impl std::error::Error for FrameToobig {}

impl fmt::Display for FrameToobig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "tryied to write too big frame")
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionClosed;

impl std::error::Error for ConnectionClosed {}

impl fmt::Display for ConnectionClosed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "connection have been close")
    }
}
