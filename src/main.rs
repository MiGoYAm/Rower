use std::net::SocketAddr;

use anyhow::anyhow;
use handlers::STATES;
use log::{error, info};
use protocol::codec::experiment::read_packets;
use protocol::codec::minecraft_codec::Connection;
use protocol::packet::handshake::Handshake;
use protocol::packet::login::{Disconnect, LoginStart, LoginSuccess, SetCompression};
use protocol::packet::PacketType;
use protocol::{Direction, LOGIN, STATUS, State};
use tokio::net::{TcpListener, TcpStream};

use crate::component::Component;

use crate::config::CONFIG;
use crate::protocol::packet::status::{Ping, StatusRequest, StatusResponse};
use crate::protocol::util::{get_string, put_str};
use crate::protocol::{generate_offline_uuid, ProtocolVersion};

mod component;
mod config;
mod handlers;
mod protocol;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;

    let listener = TcpListener::bind(CONFIG.address).await?;

    info!("Listening on {}", CONFIG.address);

    loop {
        let (stream, _) = listener.accept().await?;
        stream.set_nodelay(true)?;

        tokio::spawn(handle(stream));
    }
}

async fn handle(stream: TcpStream) {
    if let Err(err) = handle_handshake(Connection::new(stream, Direction::Serverbound)).await {
        error!("{}", err);
    }
}

async fn handle_handshake(mut client: Connection) -> anyhow::Result<()> {
    let Handshake { state, protocol, .. } = client.read_packet().await?;

    client.protocol = protocol.try_into()?;

    match state {
        STATUS => handle_status(client).await,
        LOGIN => handle_login(client).await,
        _ => Err(anyhow!("Handshake packet with wrong next state")),
    }
}

async fn handle_status(mut client: Connection) -> anyhow::Result<()> {
    client.change_state(State::Status);

    client.read_packet::<StatusRequest>().await?;

    client.write_packet(StatusResponse { status: &STATES }).await?;

    let ping: Ping = client.read_packet().await?;
    client.write_packet(ping).await
}

async fn handle_login(mut client: Connection) -> anyhow::Result<()> {
    client.change_state(State::Login);
    let LoginStart { username, .. } = client.read_packet().await?;

    if client.protocol < ProtocolVersion::V1_19_2 {
        return client.write_packet(Disconnect {
            reason: Component::text_str("We support versions above 1.19.1"),
        }).await;
    }

    if CONFIG.online {}

    if CONFIG.threshold > -1 {
        client.queue_packet(SetCompression {
            threshold: CONFIG.threshold,
        }).await?;
        client.enable_compression(CONFIG.threshold);
    }

    client.write_packet(LoginSuccess {
        username: username.clone(),
        uuid: generate_offline_uuid(&username),
    }).await?;

    let server = create_backend_connection(CONFIG.backend_server, client.protocol, username).await?;

    handle_play(client, server).await
}

async fn handle_play(mut client: Connection, mut server: Connection) -> anyhow::Result<()> {
    client.change_state(State::Play);
    server.change_state(State::Play);

    let (client_read, mut client_write, _client_info) = client.convert();
    let (server_read, mut server_write, _server_info) = server.convert();
    let mut client_recv = read_packets(client_read);
    let mut server_recv = read_packets(server_read);

    loop {
        tokio::select! {
            Some((packet, end)) = server_recv.recv() => {
                match packet {
                    PacketType::Raw(packet) => {
                        if end {
                            client_write.write_raw_packet(packet).await?
                        } else {
                            client_write.queue_raw_packet(packet).await?
                        }
                    },
                    PacketType::PluginMessage(mut packet) => {
                        if packet.channel == "minecraft:brand" {
                            let mut brand = get_string(&mut packet.data, 32700)?;
                            brand.push_str(" inside a bike");

                            packet.data.clear();
                            put_str(&mut packet.data, &brand);
                        }
                        client_write.write_packet(packet).await?;
                    },
                    PacketType::Disconnect(packet) => {
                        client_write.write_packet(packet).await?;
                        //let (server_result, client_result) = tokio::join!(server.shutdown(), client.shutdown());
                        //server_result?; client_result?;
                    },
                    _ => println!("server cos wysłał")
                }
            },
            Some((packet, end)) = client_recv.recv() => {
                match packet {
                    PacketType::Raw(packet) => {
                        if end {
                            server_write.write_raw_packet(packet).await?
                        } else {
                            server_write.queue_raw_packet(packet).await?
                        }
                    },
                    _ => println!("client cos wysłał")
                }
            },
            else => return Ok(())
        };
    }

    /* 
    loop {
        tokio::select! {
            Ok(packet) = server.next_packet() => {
                match packet {
                    PacketType::Raw(packet) => client.write_raw_packet(packet).await?,
                    PacketType::PluginMessage(mut packet) => {
                        if packet.channel == "minecraft:brand" {
                            let mut brand = get_string(&mut packet.data, 32700)?;
                            brand.push_str(" inside a bike");

                            packet.data.clear();
                            put_str(&mut packet.data, &brand);
                        }
                        client.write_packet(packet).await?;
                    },
                    PacketType::Disconnect(packet) => {
                        client.write_packet(packet).await?;
                        let (server_result, client_result) = tokio::join!(server.shutdown(), client.shutdown());
                        server_result?; client_result?;
                        std::process::exit(2);
                    },
                    _ => println!("server cos wysłał")
                }
            },
            Ok(packet) = client.next_packet() => {
                match packet {
                    PacketType::Raw(packet) => server.write_raw_packet(packet).await?,
                    _ => println!("client cos wysłał")
                }
            },
            else => return Ok(())
        };
    }
    */
}

async fn create_backend_connection(backend_server: SocketAddr, version: ProtocolVersion, username: String) -> anyhow::Result<Connection> {
    let mut server = Connection::connect(backend_server, version, Direction::Clientbound).await?;

    server.queue_packet(Handshake {
        protocol: version as i32,
        server_address: backend_server.ip().to_string(),
        port: backend_server.port(),
        state: LOGIN,
    }).await?;

    server.change_state(State::Login);
    server.write_packet(LoginStart { 
        username, 
        uuid: None 
    }).await?;

    loop {
        return match server.next_packet().await? {
            PacketType::EncryptionRequest(_) => unimplemented!("Encryption is not implemented"),
            PacketType::SetCompression(SetCompression { threshold }) => {
                server.enable_compression(threshold);
                continue;
            }
            PacketType::LoginSuccess(_) => Ok(server),
            PacketType::LoginPluginRequest(_) => Err(anyhow!("login plugin request")),
            PacketType::Disconnect(_) => Err(anyhow!("disconnect")),
            _ => Err(anyhow!("login idk")),
        };
    }
}
