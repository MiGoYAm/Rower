use handlers::STATES;
use log::{info, error};
use protocol::codec::minecraft_codec::Connection;
use protocol::codec::reg::{STATUS_REG, LOGIN_REG, PLAY_REG};
use protocol::packet::NextPacket;
use protocol::packet::handshake::Handshake;
use protocol::packet::login::{Disconnect, LoginStart, LoginSuccess};
use protocol::{LOGIN, STATUS, Direction};
use std::error::Error;
use tokio::net::{TcpListener, TcpStream};
use uuid::Uuid;

use crate::component::TextComponent;

use crate::protocol::ProtocolVersion;
use crate::protocol::packet::status::{Ping, StatusRequest, StatusResponse};

mod component;
mod error;
mod protocol;
mod handlers;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::init_with_level(log::Level::Info)?;

    const ADDRESS: &str = "127.0.0.1:25565";
    let listener = TcpListener::bind(ADDRESS).await?;

    info!("Listening on {}", ADDRESS);

    loop {
        let (stream, _) = listener.accept().await?;

        tokio::spawn(handle(stream));
    }
}

#[inline(always)]
async fn handle(mut stream: TcpStream) {
    let connection = Connection::new(&mut stream, Direction::Serverbound);

    if let Err(err) = handle_handshake(connection).await {
        error!("{}", err);
    }
}

#[inline(always)]
async fn handle_handshake(mut client: Connection<'_>) -> Result<(), Box<dyn Error>> {
    let Handshake { state, protocol, ..} = client.read_packet().await?;

    client.protocol = (protocol as u32).try_into()?;

    match state {
        STATUS => handle_status(client).await?,
        LOGIN => handle_login(client).await?,
        _ => { /* error handle*/ }
    }
    Ok(())
}

#[inline(always)]
async fn handle_status(mut client: Connection<'_>) -> Result<(), Box<dyn Error>> {
    client.set_registry(&STATUS_REG);

    client.read_packet::<StatusRequest>().await?;

    let status_response = StatusResponse { status: &STATES };
    client.write_packet(status_response).await?;

    let ping: Ping = client.read_packet().await?;
    client.write_packet(ping).await
}

#[inline(always)]
async fn handle_login(mut client: Connection<'_>) -> Result<(), Box<dyn Error>> {
    client.set_registry(&LOGIN_REG);
    let login_start: LoginStart = client.read_packet().await?;

    if false {
        let disconnect = Disconnect {
            reason: TextComponent::new(
                (if client.protocol < ProtocolVersion::V1_19_2 {
                    "We support version above 1.19.1"
                } else {
                    "ssa"
                })
                .to_string(),
            ),
        };
        client.write_packet(disconnect).await?;
        return Ok(());
    }
 
    let login_success = LoginSuccess {
        username: login_start.username,
        uuid: Uuid::new_v4(),
    };

    client.write_packet(login_success).await?;
    client.set_registry(&PLAY_REG);

    let mut server_stream = TcpStream::connect("127.0.0.1:25566").await?;
    let mut server = Connection::new(&mut server_stream, Direction::Clientbound);
    create_backend_connection(client.protocol, &mut server).await?;
    
    loop {
        tokio::select! {
            Ok(packet) = server.next_packet() => {
                match packet {
                    NextPacket::RawPacket(p) => {
                        if p.id == 0x17 {
                            println!("disconnect")
                        }
                        client.write_raw_packet(p).await?;
                    },
                    NextPacket::Disconnect(_p) => {
                        println!("odebrano");
                        let disconnect = Disconnect { reason: TextComponent::new("kys".to_string()) };
                        client.write_packet(disconnect).await?
                    },
                    _ => {}
                };
            },
            Ok(NextPacket::RawPacket(p)) = client.next_packet() 
                => server.write_raw_packet(p).await?,
        };
    }
}

async fn create_backend_connection(version: ProtocolVersion, server: &mut Connection<'_>) -> Result<(), Box<dyn Error>> {
    server.protocol = version;
    let handshake = Handshake { 
        protocol: version as i32, 
        server_address: "127.0.0.1".to_string(), 
        port: 25566, 
        state: LOGIN 
    };
    server.put_packet(handshake).await?;

    server.set_registry(&LOGIN_REG);
    let login_start = LoginStart {
        username: "tenteges".to_string(),
        uuid: None,
    };
    server.write_packet(login_start).await?;

    match server.next_packet().await.unwrap() {
        NextPacket::LoginSuccess(_p) => {
            server.set_registry(&PLAY_REG);
        },
        NextPacket::SetCompression(_p) => println!("setcompression"),
        NextPacket::Disconnect(_p) => println!("disconnect"),
        NextPacket::RawPacket(p) => {println!("rawpacket {}", p.id)},
        _ => {println!("idk");}
    }

    Ok(())
}
