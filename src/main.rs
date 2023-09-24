
use std::net::SocketAddr;

use anyhow::anyhow;
use error::ProxyError;
use handlers::STATES;
use log::{error, info};
use protocol::client::{Client, ConnectionInfo};
use protocol::codec::connection::{Connection, ReadHalf, WriteHalf};
use protocol::packet::handshake::{Handshake, NextState};
use protocol::packet::login::{Disconnect, LoginStart, LoginSuccess, SetCompression};
use protocol::packet::PacketType;
use protocol::packet::play::{JoinGame, Respawn};
use protocol::{Direction, State};
use tokio::net::{TcpListener, TcpStream};
use tokio::task;
use uuid::Uuid;

use crate::component::Component;

use crate::config::CONFIG;
use crate::protocol::packet::play::{BossBarAction, BossBar};
use crate::protocol::packet::status::{Ping, StatusRequest, StatusResponse};
use crate::protocol::util::{get_string, put_string};
use crate::protocol::{generate_offline_uuid, ProtocolVersion};

mod component;
mod config;
mod handlers;
mod protocol;
mod error;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;

    let listener = TcpListener::bind(CONFIG.address).await?;
    info!("Listening on {}", CONFIG.address);

    let local = task::LocalSet::new();
    local.run_until(listen(listener)).await
}

async fn listen(listener: TcpListener) -> anyhow::Result<()> {
    loop {
        let (stream, _) = listener.accept().await?;
        stream.set_nodelay(true)?;
    
        task::spawn_local(handle(stream));
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
    let LoginStart { username, uuid } = client.read_packet().await?;

    let mut client = Client::from_conn(client);

    if client.conn.protocol < ProtocolVersion::V1_19_2 {
        return client.disconnect(Component::text("We support versions above 1.19.1")).await;
    }

    if CONFIG.online {}

    let threshold = CONFIG.threshold;
    if threshold > -1 {
        client.conn.queue_packet(SetCompression { threshold }).await?;
        client.conn.enable_compression(threshold);
    }

    let conn_info = ConnectionInfo {
        uuid: if let Some(uuid) = uuid { uuid } else { generate_offline_uuid(&username) },
        username,
        server: CONFIG.backend_server,
        boss_bars: Vec::new()
    };

    let server = match create_backend_connection(CONFIG.backend_server, client.conn.protocol, &conn_info.username, conn_info.uuid).await {
        Ok(server) => server,
        Err(ProxyError::Disconnected(reason)) => return client.disconnect(reason).await,
        Err(ProxyError::Other(error)) => return Err(error)
    };

    client.conn.write_packet(LoginSuccess {
        uuid: conn_info.uuid,
        username: conn_info.username.clone(),
        properties: Vec::new()
    }).await?;

    handle_play(client, server, conn_info).await
}

async fn handle_play(mut client: Client, mut server: Connection, connection: ConnectionInfo) -> anyhow::Result<()> {
    client.conn.change_state(State::Play);

    let join: JoinGame = server.read_packet().await?;
    client.conn.queue_packet(join).await?;

    let client = client.conn.split();
    let server = server.split();

    let server_handle = task::spawn_local(handle_server(server.0, client.1, connection));
    let client_handle = task::spawn_local(handle_client(server.1, client.0));

    tokio::join!(client_handle, server_handle);
    Ok(())
}

async fn handle_client(mut server: WriteHalf, mut client: ReadHalf) -> anyhow::Result<()> {
    loop {
        match client.auto_read().await? {
            PacketType::Raw(packet) => {
                server.write_raw_packet(packet).await?;
                if client.is_buffer_empty() {
                    server.flush().await?
                }
            },
            _ => println!("client cos wysłał")
        }
    }
}

async fn handle_server(mut server: ReadHalf, mut client: WriteHalf, mut connection: ConnectionInfo) -> anyhow::Result<()> {
    loop {
        match server.auto_read().await? {
            PacketType::Raw(packet) => {
                client.write_raw_packet(packet).await?;
                if server.is_buffer_empty() {
                    client.flush().await?
                }
            },
            PacketType::PluginMessage(mut packet) => {
                if packet.channel == "minecraft:brand" {
                    let mut brand = get_string(&mut packet.data, 32700)?;
                    brand.push_str(" inside a bike");
    
                    packet.data.clear();
                    put_string(&mut packet.data, &brand);
                }
                client.queue_packet(packet).await?;
            },
            PacketType::Disconnect(_packet) => {
                //server.shutdown().await?;
    
                //server = create_backend_connection(CONFIG.fallback_server, client.conn.protocol, &connection.username, connection.uuid).await?;
                let join: JoinGame = server.read_packet().await?;
                let respawn = Respawn::from_joingame(&join);
                client.queue_packet(join).await?;
                client.queue_packet(respawn).await?;
    
                for uuid in &connection.boss_bars {
                    client.queue_packet(BossBar {
                        uuid: *uuid,
                        action: BossBarAction::Remove
                    }).await?;
                }
                connection.boss_bars.clear();
            },
            PacketType::BossBar(packet) => {
                match packet.action {
                    BossBarAction::Add { .. } => connection.boss_bars.push(packet.uuid),
                    BossBarAction::Remove => {
                        if let Some(index) = connection.boss_bars.iter().position(|&i| i == packet.uuid) {
                            connection.boss_bars.swap_remove(index);
                        }
                    }
                    _ => {}
                }
                client.queue_packet(packet).await?;
            }
            _ => println!("server cos wysłał")
        }
    }

}

async fn create_backend_connection(backend_server: SocketAddr, version: ProtocolVersion, username: &str, uuid: Uuid) -> Result<Connection, ProxyError> {
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
        uuid: Some(uuid)
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
