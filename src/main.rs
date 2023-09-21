use std::net::SocketAddr;

use anyhow::anyhow;
use error::ProxyError;
use handlers::STATES;
use log::{error, info};
use protocol::client::Client;
use protocol::codec::connection::Connection;
use protocol::packet::handshake::{Handshake, NextState};
use protocol::packet::login::{Disconnect, LoginStart, LoginSuccess, SetCompression};
use protocol::packet::PacketType;
use protocol::packet::play::{JoinGame, Respawn};
use protocol::{Direction, State};
use tokio::net::{TcpListener, TcpStream};

use crate::component::Component;

use crate::config::CONFIG;
use crate::protocol::packet::status::{Ping, StatusRequest, StatusResponse};
use crate::protocol::util::{get_string, put_string};
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

    let mut client = Client::from_conn(client);

    if client.conn.protocol < ProtocolVersion::V1_19_2 {
        return client.disconnect(Component::text_str("We support versions above 1.19.1")).await;
    }

    if CONFIG.online {}

    let threshold = CONFIG.threshold;
    if threshold > -1 {
        client.conn.queue_packet(SetCompression { threshold }).await?;
        client.conn.enable_compression(threshold);
    }

    let server = match create_backend_connection(CONFIG.backend_server, client.conn.protocol, &username).await {
        Ok(server) => server,
        Err(ProxyError::Disconnected(reason)) => return client.disconnect(reason).await,
        Err(ProxyError::Other(error)) => return Err(error)
    };

    client.conn.write_packet(LoginSuccess {
        uuid: generate_offline_uuid(&username),
        username,
        properties: Vec::new()
    }).await?;

    handle_play(client, server).await
}

async fn handle_play(mut client: Client, mut server: Connection) -> anyhow::Result<()> {
    client.conn.change_state(State::Play);

    let join: JoinGame = server.read_packet().await?;
    client.conn.queue_packet(join).await?;
    
    loop {
        tokio::select! {
            Ok(packet) = server.auto_read() => {
                match packet {
                    PacketType::Raw(packet) => client.conn.write_raw_packet(packet).await?,
                    PacketType::PluginMessage(mut packet) => {
                        if packet.channel == "minecraft:brand" {
                            let mut brand = get_string(&mut packet.data, 32700)?;
                            brand.push_str(" inside a bike");

                            packet.data.clear();
                            put_string(&mut packet.data, &brand);
                        }
                        client.conn.queue_packet(packet).await?;
                    },
                    PacketType::Disconnect(_packet) => {
                        server.shutdown().await?;

                        server = create_backend_connection(CONFIG.fallback_server, client.conn.protocol, "temp").await?;
                        let join: JoinGame = server.read_packet().await?;
                        let respawn = Respawn::from_joingame(&join);
                        client.conn.queue_packet(join).await?;
                        client.conn.queue_packet(respawn).await?;
                    },
                    _ => println!("server cos wysłał")
                }
            },
            Ok(packet) = client.conn.auto_read() => {
                match packet {
                    PacketType::Raw(packet) => server.write_raw_packet(packet).await?,
                    _ => println!("client cos wysłał")
                }
            },
            else => return Ok(())
        };
    }
}

async fn create_backend_connection(backend_server: SocketAddr, version: ProtocolVersion, username: &str) -> Result<Connection, ProxyError> {
    let mut server = Connection::connect(backend_server, version, Direction::Clientbound).await?;

    server.queue_packet(Handshake {
        protocol: version.into(),
        server_address: backend_server.ip().to_string(),
        port: backend_server.port(),
        state: NextState::Login,
    }).await?;

    server.change_state(State::Login);
    server.write_packet(LoginStart { 
        username: username.to_owned(), 
        uuid: None 
    }).await?;

    loop {
        return match server.auto_read().await? {
            PacketType::EncryptionRequest(_) => unimplemented!("Encryption is not implemented"),
            PacketType::SetCompression(SetCompression { threshold }) => {
                server.enable_compression(threshold);
                continue;
            }
            PacketType::LoginSuccess(_) => {
                server.change_state(State::Play);
                Ok(server)
            }
            PacketType::LoginPluginRequest(_) => unimplemented!("login plugin request"),
            PacketType::Disconnect(Disconnect { reason }) => {
                server.shutdown().await?;
                Err(ProxyError::Disconnected(reason))
            },
            _ => Err(anyhow!("unknown packet").into()),
        };
    }
}
