use std::error::Error;

use tokio_util::codec::{Encoder};

use crate::protocol::{packet::Packet, V1_19_4};

struct CompressionCodec;

impl Encoder<&dyn Packet> for CompressionCodec {
    type Error = Box<dyn Error>;

    fn encode(&mut self, item: &dyn Packet, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        item.put_buf(dst, V1_19_4);
        todo!()
    }
}