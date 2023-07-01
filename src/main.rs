use bytes::BytesMut;
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
use uuid::Uuid;

use crate::component::{Component};

use crate::protocol::ProtocolVersion;
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
    let connection = Connection::new(stream, Direction::Serverbound);

    if let Err(err) = handle_handshake(connection).await {
        error!("{}", err);
    }
}

async fn handle_handshake(mut client: Connection) -> Result<(), Box<dyn Error>> {
    let Handshake { state, protocol, ..} = client.read_packet().await?;

    match state {
        STATUS => handle_status(client).await?,
        LOGIN => {
            client.protocol = (protocol as u32).try_into()?;
            handle_login(client).await?;
        },
        _ => { /* error handle */ }
    }
    Ok(())
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

    if client.protocol < ProtocolVersion::V1_19_2  {
        return client.write_packet(Disconnect {
            reason: Component::text("We support versions above 1.19.1".to_string())
        }).await;
    }

    if config::ONLINE {}

    if config::THRESHOLD > -1 {
        client.write_packet(SetCompression { threshold: config::THRESHOLD }).await?;
        client.enable_compression(config::THRESHOLD as u32);
    }
 
    client.write_packet(LoginSuccess {
        username: username.clone(),
        uuid: Uuid::new_v4(),
    }).await?;

    let initial_server = &config::SERVERS[0];
    let server = create_backend_connection(initial_server, client.protocol, username).await?;
    
    handle_play(client, server).await
}

async fn handle_play(mut client: Connection, mut server: Connection) -> Result<(), Box<dyn Error>>{
    client.change_state(PLAY);
    loop {
        tokio::select! {
            Ok(packet) = server.next_packet() => {
                match packet {
                    PacketType::Raw(p) => client.write_raw_packet(p).await?,
                    PacketType::PluginMessage(mut p) => {
                        let mut packet = p.get()?;

                        if packet.channel == "minecraft:brand" {
                            let mut buf = BytesMut::from(packet.data.as_slice());

                            let mut brand = get_string(&mut buf, 32700)?;
                            brand.push_str(" inside a bike");

                            buf.clear();
                            put_str(&mut buf, &brand);

                            packet.data = buf.to_vec();
                        }
                        client.write_packet(packet).await?;
                    },
                    PacketType::Disconnect(mut p) => {
                        let packet = p.get()?;

                        server.shutdown().await?;
                        client.write_packet(packet).await?;
                        client.shutdown().await?;
                    },
                    _ => println!("server cos wysłał")
                };
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

async fn create_backend_connection(backend_server: &Server, version: ProtocolVersion, username: String) -> Result<Connection, Box<dyn Error>> {
    let mut server = Connection::connect(backend_server.ip, Direction::Clientbound).await?;

    server.protocol = version;

    server.queue_packet(Handshake { 
        protocol: version as i32, 
        server_address: "127.0.0.1".to_string(), 
        port: 25566, 
        state: LOGIN 
    }).await?;

    server.change_state(LOGIN);
    server.write_packet(LoginStart {
        username,
        uuid: None,
    }).await?;

    match server.next_packet().await? {
        PacketType::EncryptionRequest(_) => panic!("encryption not implemented"),
        PacketType::SetCompression(mut p) => {
            let threshold = p.get()?.threshold;
            if threshold > -1 {
                server.enable_compression(threshold as u32);
            }

            let _p: LoginSuccess = server.read_packet().await?;
            server.change_state(PLAY);
        },
        PacketType::LoginSuccess(_) => server.change_state(PLAY),
        PacketType::Disconnect(_) => println!("disconnect"),
        _ => println!("login idk")
    }

    Ok(server)
}
