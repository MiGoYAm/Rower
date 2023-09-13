use std::net::SocketAddr;

use anyhow::anyhow;
use error::ProxyError;
use handlers::{STATES, get_initial_server};
use log::{error, info};
use protocol::codec::minecraft_codec::Connection;
use protocol::packet::handshake::{Handshake, NextState};
use protocol::packet::login::{Disconnect, LoginStart, LoginSuccess, SetCompression};
use protocol::packet::PacketType;
use protocol::{Direction, State};
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
mod error;

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
        NextState::Status => handle_status(client).await,
        NextState::Login => handle_login(client).await,
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
        return client.disconnect(Component::text_str("We support versions above 1.19.1")).await;
    }

    if CONFIG.online {}

    let threshold = CONFIG.threshold;
    if threshold > -1 {
        client.queue_packet(SetCompression { threshold }).await?;
        client.enable_compression(threshold);
    }

    let server = match create_backend_connection(CONFIG.backend_server, client.protocol, &username).await {
        Ok(server) => server,
        Err(ProxyError::ServerDisconnected(reason)) => return client.disconnect(reason).await,
        Err(ProxyError::Other(e)) => return Err(e)
    };

    client.write_packet(LoginSuccess {
        uuid: generate_offline_uuid(&username),
        username
    }).await?;

    handle_play(client, server).await
}

async fn handle_play(mut client: Connection, mut server: Connection) -> anyhow::Result<()> {
    client.change_state(State::Play);
    server.change_state(State::Play);
    
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
                        tokio::try_join!(server.shutdown(), client.shutdown())?;
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
}

async fn create_backend_connection(backend_server: SocketAddr, version: ProtocolVersion, username: &String) -> Result<Connection, ProxyError> {
    let mut server = Connection::connect(backend_server, version, Direction::Clientbound).await?;

    server.queue_packet(Handshake {
        protocol: version as i32,
        server_address: backend_server.ip().to_string(),
        port: backend_server.port(),
        state: NextState::Login,
    }).await?;

    server.change_state(State::Login);
    server.write_packet(LoginStart { 
        username: username.clone(), 
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
            PacketType::LoginPluginRequest(_) => unimplemented!("login plugin request"),
            PacketType::Disconnect(Disconnect { reason }) => {
                server.shutdown().await?;
                Err(ProxyError::ServerDisconnected(reason))
            },
            _ => Err(anyhow!("unknown packet").into()),
        };
    }
}
