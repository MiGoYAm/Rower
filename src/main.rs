use config::Server;
use handlers::STATES;
use log::{info, error};
use protocol::codec::minecraft_codec::Connection;
use protocol::packet::PacketType;
use protocol::packet::handshake::Handshake;
use protocol::packet::login::{Disconnect, LoginStart, LoginSuccess, SetCompression};
use protocol::{LOGIN, STATUS, Direction, PLAY};
use std::error::Error;
use tokio::net::{TcpListener, TcpStream};

use crate::component::Component;

use crate::protocol::{ProtocolVersion, generate_offline_uuid};
use crate::protocol::packet::status::{Ping, StatusRequest, StatusResponse};
use crate::protocol::util::{put_str, get_string};

mod component;
mod error;
mod protocol;
mod handlers;
mod config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {    
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

async fn handle_handshake(mut client: Connection) -> Result<(), Box<dyn Error>> {
    let Handshake { state, protocol, .. } = client.read_packet().await?;

    client.protocol = protocol.try_into()?;

    match state {
        STATUS => handle_status(client).await,
        LOGIN => handle_login(client).await,
        _ => Err("handshake packet with wrong next state".into())
    }
}

async fn handle_status(mut client: Connection) -> Result<(), Box<dyn Error>> {
    client.change_state(STATUS);

    client.read_packet::<StatusRequest>().await?;

    client.write_packet(StatusResponse { status: &STATES }).await?;

    let ping: Ping = client.read_packet().await?;
    client.write_packet(ping).await
}

async fn handle_login(mut client: Connection) -> Result<(), Box<dyn Error>> {
    client.change_state(LOGIN);
    let LoginStart { username, .. } = client.read_packet().await?;

    if client.protocol < ProtocolVersion::V1_19_2 {
        return client.write_packet(Disconnect {
            reason: Component::text_str("We support versions above 1.19.1")
        }).await
    }

    if config::ONLINE {}

    if config::THRESHOLD > -1 {
        client.queue_packet(SetCompression { threshold: config::THRESHOLD }).await?;
        client.enable_compression(config::THRESHOLD as u32);
    }
 
    client.write_packet(LoginSuccess {
        username: username.clone(),
        uuid: generate_offline_uuid(&username),
    }).await?;

    let initial_server = &config::SERVERS[0];
    let server = create_backend_connection(initial_server, client.protocol, username).await?;
    
    handle_play(client, server).await
}

async fn handle_play(mut client: Connection, mut server: Connection) -> Result<(), Box<dyn Error>> {
    client.change_state(PLAY);
    loop {
        tokio::select! {
            Some(packet) = server.next_packet() => {
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
                        server.shutdown().await?;
                        client.write_packet(packet).await?;
                        client.shutdown().await?;
                    },
                    _ => println!("server cos wysłał")
                }
            },
            Some(packet) = client.next_packet() => { 
                match packet {
                    PacketType::Raw(p) => server.write_raw_packet(p).await?,
                    _ => println!("client cos wysłał")
                }
            },  
            else => return Ok(())
        };
    }
}

async fn create_backend_connection(backend_server: &Server, version: ProtocolVersion, username: String) -> Result<Connection, Box<dyn Error>> {
    let mut server = Connection::connect(backend_server.address, version, Direction::Clientbound).await?;

    server.queue_packet(Handshake { 
        protocol: version as i32, 
        server_address: backend_server.address.ip().to_string(), 
        port: backend_server.address.port(), 
        state: LOGIN 
    }).await?;

    server.change_state(LOGIN);
    server.write_packet(LoginStart {
        username,
        uuid: None,
    }).await?;

    match server.next_packet().await.unwrap() {
        PacketType::EncryptionRequest(_) => panic!("encryption is not implemented"),
        PacketType::SetCompression(SetCompression { threshold }) => {
            if threshold > -1 {
                server.enable_compression(threshold as u32);
            }

            server.read_packet::<LoginSuccess>().await?;
            server.change_state(PLAY);
        },
        PacketType::LoginSuccess(_) => server.change_state(PLAY),
        PacketType::Disconnect(_) => println!("disconnect"),
        _ => println!("login idk")
    }

    Ok(server)
}
