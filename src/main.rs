#![feature(lazy_cell)]
use std::future::Future;
use std::net::SocketAddr;

use anyhow::{anyhow, Result};
use error::ProxyError;
use handlers::{STATUS, get_initial_server};
use log::{error, info};
use protocol::buffer::{BufExt, BufMutExt};
use protocol::wrappers::{Client, ConnectionInfo, Server};
use protocol::codec::connection::{Connection, ReadHalf, WriteHalf};
use protocol::packet::handshake::{Handshake, NextState};
use protocol::packet::login::{Disconnect, LoginStart, LoginSuccess, SetCompression};
use protocol::packet::PacketType;
use protocol::packet::play::{JoinGame, Respawn, BossBar};
use protocol::{Direction, State};
use tokio::net::TcpListener;
use tokio::task::{self, JoinHandle};

use crate::component::Component;

use crate::config::{CONFIG, load_config};
use crate::handlers::create_status;
use crate::protocol::packet::play::BossBarAction;
use crate::protocol::packet::status::{Ping, StatusRequest, StatusResponse};
use crate::protocol::{generate_offline_uuid, ProtocolVersion};

mod component;
mod config;
mod handlers;
mod protocol;
mod error;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;
    CONFIG.set(load_config()?).unwrap();
    STATUS.set(create_status()?).unwrap();

    let address = CONFIG.get().unwrap().address;
    let listener = TcpListener::bind(address).await?;
    info!("Listening on {}", address);

    let local = task::LocalSet::new();
    local.run_until(listen(listener)).await
}

async fn listen(listener: TcpListener) -> Result<()> {
    loop {
        let (stream, _) = listener.accept().await?;
        stream.set_nodelay(true)?;
        spawn_handle(|| handle_handshake(Connection::new(stream, Direction::Clientbound)));
    }
}

fn spawn_handle<F, Fut>(f: F) -> JoinHandle<()>
where
    F: FnOnce() -> Fut + 'static,
    Fut: Future<Output = Result<()>>
{
    task::spawn_local(async move {
        if let Err(err) = f().await {
            error!("{}", err);
        }
    })
}

async fn handle_handshake(mut client: Connection) -> Result<()> {
    let Handshake { state, protocol, .. } = client.read_packet().await?;

    client.protocol = protocol.into();

    match state {
        NextState::Status => handle_status(client).await,
        NextState::Login => handle_login(client).await,
    }
}

async fn handle_status(mut client: Connection) -> Result<()> {
    client.change_state(State::Status);

    client.read_packet::<StatusRequest>().await?;

    client.write_packet(StatusResponse { status: STATUS.get().unwrap() }).await?;

    let ping: Ping = client.read_packet().await?;
    client.write_packet(ping).await
}

async fn handle_login(mut client: Connection) -> Result<()> {
    client.change_state(State::Login);
    let LoginStart { username, uuid } = client.read_packet().await?;

    let mut client = Client::new(client);

    if client.conn.protocol < ProtocolVersion::V1_19_2 {
        return client.disconnect(Component::text("We support versions above 1.19.1")).await;
    }

    if CONFIG.get().unwrap().online {}

    let threshold = CONFIG.get().unwrap().compression_threshold;
    if threshold > -1 {
        client.conn.queue_packet(SetCompression { threshold }).await?;
        client.conn.enable_compression(threshold);
    }

    let uuid = uuid.unwrap_or_else(|| generate_offline_uuid(&username));
    let conn_info = ConnectionInfo::new(username, uuid);
    let initial_server = get_initial_server();

    let server = match create_backend_connection(initial_server, client.conn.protocol, &conn_info).await {
        Ok(server) => server,
        Err(ProxyError::Disconnected(reason)) => return client.disconnect(reason).await,
        Err(ProxyError::Other(error)) => return Err(error)
    };

    client.conn.write_packet(LoginSuccess {
        uuid: conn_info.uuid,
        username: conn_info.username.clone(),
        properties: vec![]
    }).await?;

    handle_play(client, server, conn_info).await
}

async fn handle_play(mut client: Client, server: Server, connection: ConnectionInfo) -> Result<()> {
    client.conn.change_state(State::Play);

    let (client_read, client_write) = client.conn.split();
    let (server_read, server_write) = server.conn.split();

    let _server_handle = spawn_handle(|| handle_server(server_read, client_write, connection));
    let _client_handle = spawn_handle(|| handle_client(client_read, server_write));

    Ok(())
}


async fn handle_client(mut client: ReadHalf, mut server: WriteHalf) -> Result<()> {
    loop {
        match client.auto_read().await? {
            PacketType::ChatCommand(packet) => {
                println!("chat command");
                if packet.command == "switch" {

                }
                server.queue_packet(packet).await?;
            }
            PacketType::Raw(packet) => {
                server.queue_raw_packet(packet).await?;
                if client.is_buffer_empty() {
                    server.flush().await?
                }
            },
            _ => unreachable!("client cos wysłał")
        }
    }
}

async fn handle_server(mut server: ReadHalf, mut client: WriteHalf, mut connection: ConnectionInfo) -> Result<()> {
    loop {
        match server.auto_read().await? {
            PacketType::PluginMessage(mut packet) => {
                if packet.channel == "minecraft:brand" {
                    let mut brand = packet.data.get_string(32700)?;
                    brand.push_str(" inside a bike");
    
                    packet.data.clear();
                    packet.data.put_string(&brand);
                }
                client.queue_packet(packet).await?;
            },
            PacketType::Disconnect(packet) => {
                // todo: close server connection
                return client.write_packet(packet).await;
                //return client.shutdown().await;
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
            },
            PacketType::Raw(packet) => {
                client.queue_raw_packet(packet).await?;
                if server.is_buffer_empty() {
                    client.flush().await?;
                }
            },
            _ => unreachable!("server cos wysłał")
        }
    }

}

async fn create_backend_connection(server_address: SocketAddr, version: ProtocolVersion, connection: &ConnectionInfo) -> Result<Server, ProxyError> {
    let mut server = Connection::connect(server_address, version, Direction::Serverbound).await?;

    server.queue_packet(Handshake {
        protocol: version.into(),
        server_address: server_address.ip().to_string(),
        port: server_address.port(),
        state: NextState::Login,
    }).await?;

    server.change_state(State::Login);
    server.write_packet(LoginStart { 
        username: connection.username.clone(), 
        uuid: Some(connection.uuid)
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
                Ok(Server::new(server, server_address))
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

async fn switch_server(client: &mut WriteHalf, server_address: SocketAddr, connection: &mut ConnectionInfo) -> Result<Server> {
    let mut server = create_backend_connection(server_address, client.protocol, connection).await?;
    let join: JoinGame = server.conn.read_packet().await?;
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
    Ok(server)
}
