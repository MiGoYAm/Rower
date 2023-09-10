use futures::StreamExt;
use tokio::sync::mpsc::{self, UnboundedReceiver};

use crate::protocol::packet::PacketType;

use super::minecraft_codec::Read;

pub fn read_packets(decoder: Read) -> UnboundedReceiver<(PacketType<'static>, bool)> {
    let (sender, receiver) = mpsc::unbounded_channel();
    tokio::spawn(read_packets_loop(decoder, sender));
    receiver
}

async fn read_packets_loop(mut decoder: Read, channel: mpsc::UnboundedSender<(PacketType<'_>, bool)>) {

    while let Some(frame) = decoder.read.next().await {
        let frame = frame.unwrap();

        let packet = decoder.registry.decode(frame, decoder.protocol).unwrap();

        channel.send((packet, decoder.read.read_buffer().is_empty())).unwrap();
    }
}
