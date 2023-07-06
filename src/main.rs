use anyhow::anyhow;
use config::Server;
use handlers::STATES;
use log::{error, info};
use protocol::codec::minecraft_codec::Connection;
use protocol::packet::handshake::Handshake;
use protocol::packet::login::{Disconnect, LoginStart, LoginSuccess, SetCompression};
use protocol::packet::PacketType;
use protocol::{Direction, LOGIN, STATUS, State};
use tokio::net::{TcpListener, TcpStream};

use crate::component::Component;

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

    let listener = TcpListener::bind(config::ADDRESS).await?;

    info!("Listening on {}", config::ADDRESS);

    loop {
        let (stream, _) = listener.accept().await?;

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

    if config::ONLINE {}

    if config::THRESHOLD > -1 {
        client.queue_packet(SetCompression {
            threshold: config::THRESHOLD,
        }).await?;
        client.enable_compression(config::THRESHOLD);
    }

    client.write_packet(LoginSuccess {
        username: username.clone(),
        uuid: generate_offline_uuid(&username),
    }).await?;

    let initial_server = &config::SERVERS[0];
    let server = create_backend_connection(initial_server, client.protocol, username).await?;

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
                        let (server_result, client_result) = tokio::join!(server.shutdown(), client.shutdown());
                        server_result?; client_result?;
                    },
                    _ => println!("server cos wysłał")
                }
            },
            Ok(packet) = client.next_packet() => {
                match packet {
                    PacketType::Raw(p) => server.write_raw_packet(p).await?,
                    _ => println!("client cos wysłał")
                }
            },
            else => return Ok(())
        };
    }
}

async fn create_backend_connection(backend_server: &Server, version: ProtocolVersion, username: String) -> anyhow::Result<Connection> {
    let mut server = Connection::connect(backend_server.address, version, Direction::Clientbound).await?;

    server.queue_packet(Handshake {
        protocol: version as i32,
        server_address: backend_server.address.ip().to_string(),
        port: backend_server.address.port(),
        state: LOGIN,
    }).await?;

    server.change_state(State::Login);
    server.write_packet(LoginStart { 
        username, 
        uuid: None 
    }).await?;

    loop {
        return match server.next_packet().await? {
            PacketType::EncryptionRequest(_) => Err(anyhow!("Encryption is not implemented")),
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
