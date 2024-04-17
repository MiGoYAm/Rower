#![feature(lazy_cell)]
#![feature(inline_const)]
#![feature(once_cell_try)]
use std::future::Future;
use std::net::SocketAddr;

use anyhow::{ensure, Result};
use bytes::BytesMut;
use error::ProxyError;
use handlers::{get_initial_server, status};
use log::{error, info};
use online::decrypt;
use openssl::encrypt::Decrypter;
use openssl::rsa::Padding;
use protocol::buffer::{BufExt, BufMutExt};
use protocol::codec::connection::{
    ClientConn, ClientSideConn, Connection, ServerConn, ServerSideConn,
};
use protocol::packet::handshake::{Handshake, NextState};
use protocol::packet::login::{
    Disconnect, EncryptionRequest, EncryptionResponse, LoginPackets, LoginStart, LoginSuccess,
    SetCompression,
};
use protocol::packet::PacketType;
use protocol::wrappers::{ConnectionInfo, Server};
use protocol::State;
use reqwest::{StatusCode, Url};
use tokio::net::TcpListener;
use tokio::task::{self, JoinHandle};

use crate::component::Component;

use crate::config::config;
use crate::online::{generate_server_id, GameProfile, RSA_KEYS};
use crate::protocol::packet::play::BossBarAction;
use crate::protocol::packet::status::{Ping, StatusRequest, StatusResponse};
use crate::protocol::ProtocolVersion;

mod component;
mod config;
mod error;
mod handlers;
mod online;
mod protocol;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    simple_logger::init_with_level(log::Level::Info)?;

    let address = config().address;
    let listener = TcpListener::bind(address).await?;
    info!("Listening on {}", address);

    //let local = task::LocalSet::new();
    //local.run_until(listen(listener)).await
    listen(listener).await
}

async fn listen(listener: TcpListener) -> Result<()> {
    loop {
        let (stream, _) = listener.accept().await?;
        stream.set_nodelay(true)?;
        spawn(handle_handshake(Connection::new(stream)));
    }
}

fn spawn(task: impl Future<Output = Result<()>> + 'static + Send) -> JoinHandle<()> {
    task::spawn(async move {
        if let Err(err) = task.await {
            error!("{}", err);
            err.chain()
                .skip(1)
                .for_each(|cause| error!("because: {}", cause));
        }
    })
}

async fn handle_handshake(mut client: ClientConn<{ State::HANDSHAKE }>) -> Result<()> {
    let Handshake {
        state, protocol, ..
    } = client.recv_packet().await?;

    client.protocol = protocol.into();

    match state {
        NextState::Status => handle_status(client.upgrade()).await,
        NextState::Login => handle_login(client.upgrade()).await,
    }
}

async fn handle_status(mut client: ClientConn<{ State::STATUS }>) -> Result<()> {
    client.recv_packet::<StatusRequest>().await?;

    client
        .send_packet(StatusResponse { status: status() })
        .await?;

    let ping: Ping = client.recv_packet().await?;
    client.send_packet(ping).await
}

async fn handle_login(mut client: ClientConn<{ State::LOGIN }>) -> Result<()> {
    let LoginStart { username, uuid } = client.recv_packet().await?;

    if client.protocol < ProtocolVersion::V1_19_2 {
        return client
            .disconnect(Component::text("We support versions above 1.19.1"))
            .await;
    }

    if config().online {
        let mut decrypter = Decrypter::new(&RSA_KEYS.pair_key)?;
        decrypter.set_rsa_padding(Padding::PKCS1)?;

        let server_verify_token = rand::random();

        client
            .send_packet(EncryptionRequest {
                server_id: String::new(),
                public_key: RSA_KEYS.public_key.to_owned(),
                verify_token: server_verify_token,
            })
            .await?;

        let EncryptionResponse {
            shared_secret,
            verify_token,
        } = client.recv_packet().await?;

        let verify_token = decrypt(&mut decrypter, &verify_token)?;
        ensure!(verify_token == server_verify_token, "Invalid verify token");

        let shared_secret: [u8; 16] = decrypt(&mut decrypter, &shared_secret)?
            .as_slice()
            .try_into()?;

        let http_client = reqwest::Client::new();
        let server_id = generate_server_id(&shared_secret, &RSA_KEYS.public_key)?;
        let url = Url::parse_with_params(
            "https://sessionserver.mojang.com/session/minecraft/hasJoined",
            &[("username", &username), ("serverId", &server_id)],
        )?;

        let responose = http_client.get(url).send().await?.error_for_status()?;

        match responose.status() {
            StatusCode::OK => {
                let _profile: GameProfile = responose.json().await?;
            }
            StatusCode::NO_CONTENT => {
                return client
                    .disconnect(Component::text("Server is in online mode"))
                    .await
            }
            _ => {
                return client
                    .disconnect(Component::text("Failed to authenticate with Mojang"))
                    .await
            }
        }

        client.enable_encryption(shared_secret)?;
    }

    let threshold = config().compression_threshold;
    if threshold > -1 {
        client.queue_packet(SetCompression { threshold }).await?;
        client.enable_compression(threshold as u32);
    }

    let conn_info = ConnectionInfo::new(username, uuid);
    let initial_server = get_initial_server();

    let server = match create_backend_conn(initial_server, client.protocol, &conn_info).await {
        Ok(server) => server,
        Err(ProxyError::Disconnected(reason)) => return client.disconnect(reason).await,
        Err(ProxyError::Other(error)) => return Err(error),
    };

    client
        .send_packet(LoginSuccess {
            uuid: conn_info.uuid,
            username: conn_info.username.clone(),
            properties: Vec::new(),
        })
        .await?;

    handle_play(client.upgrade(), server, conn_info).await
}

async fn handle_play(
    client: ClientConn<{ State::PLAY }>,
    server: Server<{ State::PLAY }>,
    connection: ConnectionInfo,
) -> Result<()> {
    let (client_side, server_side) = client.mix(server.conn);

    let _server_handle = spawn(handle_server(client_side, connection));
    let _client_handle = spawn(handle_client(server_side));

    Ok(())
}

async fn handle_client(mut conn: ServerSideConn) -> Result<()> {
    loop {
        match conn.auto_read().await? {
            PacketType::ChatCommand(packet) => {
                println!("chat command");
                conn.queue_packet_dyn(packet).await?;
            }
            PacketType::Raw(packet) => {
                conn.queue_raw_packet(packet).await?;
                //if client.is_buffer_empty() {
                //    server.flush().await?
                //}
            }
            _ => unreachable!("client cos wysłał"),
        }
    }
}

async fn handle_server(mut conn: ClientSideConn, mut connection: ConnectionInfo) -> Result<()> {
    loop {
        match conn.auto_read().await? {
            PacketType::PluginMessage(mut packet) => {
                if packet.channel == "minecraft:brand" {
                    let mut brand = packet.data.get_string(32700)?;
                    brand.push_str(" inside a bike");

                    let mut bytes = BytesMut::with_capacity(brand.len());
                    bytes.put_string(&brand);
                    packet.data = bytes.freeze();
                }
                conn.queue_packet_dyn(packet).await?;
            }
            PacketType::Disconnect(packet) => {
                // todo: close server connection
                return conn.send_packet_dyn(packet).await;
                //return client.shutdown().await;
            }
            PacketType::BossBar(packet) => {
                match packet.action {
                    BossBarAction::Add { .. } => connection.boss_bars.push(packet.uuid),
                    BossBarAction::Remove => {
                        if let Some(index) =
                            connection.boss_bars.iter().position(|&i| i == packet.uuid)
                        {
                            connection.boss_bars.swap_remove(index);
                        }
                    }
                    _ => {}
                }
                conn.queue_packet_dyn(packet).await?;
            }
            PacketType::Raw(packet) => {
                conn.queue_raw_packet(packet).await?;
                //if server.is_buffer_empty() {
                //    client.flush().await?;
                //}
            }
            _ => unreachable!("server cos wysłał"),
        }
    }
}

async fn create_backend_conn(
    server_address: SocketAddr,
    version: ProtocolVersion,
    connection: &ConnectionInfo,
) -> Result<Server<{ State::PLAY }>, ProxyError> {
    let mut server =
        ServerConn::<{ State::HANDSHAKE }>::connect_to(server_address, version).await?;

    server
        .queue_packet(Handshake {
            protocol: version.into(),
            server_address: server_address.ip().to_string(),
            port: server_address.port(),
            state: NextState::Login,
        })
        .await?;

    let mut server = server.upgrade::<{ State::LOGIN }>();
    server
        .send_packet(LoginStart {
            username: connection.username.clone(),
            uuid: Some(connection.uuid),
        })
        .await?;

    loop {
        return match server.recv_packets().await? {
            LoginPackets::EncryptionRequest(_) => unimplemented!("Encryption is not implemented"),
            LoginPackets::SetCompression(SetCompression { threshold }) => {
                if threshold > -1 {
                    server.enable_compression(threshold as u32);
                }
                continue;
            }
            LoginPackets::LoginSuccess(_) => Ok(Server::new(server.upgrade(), server_address)),
            LoginPackets::LoginPluginRequest(_) => unimplemented!("login plugin request"),
            LoginPackets::Disconnect(Disconnect { reason }) => {
                server.shutdown().await?;
                Err(ProxyError::Disconnected(reason))
            }
        };
    }
}
/*
async fn switch_server(
    client: &mut WriteHalf,
    server_address: SocketAddr,
    connection: &mut ConnectionInfo,
) -> Result<Server<{ State::PLAY }>> {
    let mut server = create_backend_conn(server_address, client.protocol, connection).await?;
    let join: JoinGame = server.conn.recv_packet_dyn().await?;
    let respawn = Respawn::from_joingame(&join);
    client.queue_packet(join).await?;
    client.queue_packet(respawn).await?;

    for uuid in &connection.boss_bars {
        client
            .queue_packet(BossBar {
                uuid: *uuid,
                action: BossBarAction::Remove,
            })
            .await?;
    }
    connection.boss_bars.clear();
    Ok(server)
}
*/
