use protocol::codec::minecraft_codec::Connection;
use protocol::codec::registry::{STATUS_REGISTRY, LOGIN_REGISTRY, PLAY_REGISTRY};
use protocol::packet::NextPacket;
use protocol::packet::handshake::Handshake;
use protocol::packet::login::{Disconnect, LoginStart, LoginSuccess};
use protocol::{LOGIN, STATUS, V1_19_2, Direction};
use std::error::Error;
use tokio::net::{TcpListener, TcpStream};

use uuid::Uuid;

use crate::component::TextComponent;

use crate::protocol::packet::status::{Ping, Players, StatusRequest, StatusResponse, Version};

mod component;
mod error;
mod protocol;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    const ADDR: &str = "127.0.0.1:25565";
    let listener = TcpListener::bind(ADDR).await?;

    loop {
        let (stream, _) = listener.accept().await?;

        tokio::spawn(handle(stream));
    }
}

async fn handle(socket: TcpStream) {
    let mut connection = Connection::new(socket, Direction::Serverbound);

    let result: Result<(), Box<dyn Error>> = async move {
        let handshake: Handshake = connection.read_packet().await?;

        connection.protocol = handshake.protocol;

        match handshake.state {
            STATUS => handle_status(&mut connection).await?,
            LOGIN => handle_login(&mut connection).await?,
            _ => { /* error handle*/ }
        }

        connection.shutdown().await?;
        Ok(())
    }
    .await;

    if let Err(err) = result {
        println!("{}", err);
    }
}

async fn handle_status(connection: &mut Connection) -> Result<(), Box<dyn Error>> {
    connection.set_registry(&STATUS_REGISTRY);

    connection.read_packet::<StatusRequest>().await?;

    let status = StatusResponse {
        version: Version {
            name: "1.19.4",
            protocol: 762,
        },
        players: Players { online: 2, max: 16 },
        description: TextComponent::new("azz".to_string()),
        previews_chat: false,
        enforces_secure_chat: false,
    };
    connection.write_packet(status).await?;

    let ping: Ping = connection.read_packet().await?;
    connection.write_packet(ping).await?;

    Ok(())
}

async fn handle_login(client: &mut Connection) -> Result<(), Box<dyn Error>> {
    client.set_registry(&LOGIN_REGISTRY);
    let login_start: LoginStart = client.read_packet().await?;

    if false {
        let disconnect = Disconnect {
            reason: TextComponent::new(
                (if client.protocol < V1_19_2 {
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

    //println!("serwer");
    let mut server = handle_join(client.protocol).await?;
    
    client.set_registry(&PLAY_REGISTRY);
    server.set_registry(&PLAY_REGISTRY);

    /*
    loop {
        tokio::select! {
            Ok(NextPacket::RawPacket(p)) = server.next_packet() => {
                client.write_raw_packet(p).await;
            },
            Ok(NextPacket::RawPacket(p)) = client.next_packet() => {
                server.write_raw_packet(p).await;
            },
        };
    }
    */

    Ok(())
}

async fn handle_join(version: i32) -> Result<Connection, Box<dyn Error>> {
    let stream = TcpStream::connect("127.0.0.1:25566").await?;
    let mut server = Connection::new(stream, Direction::Clientbound);

    let handshake = Handshake { 
        protocol: version, 
        server_address: "127.0.0.1".to_string(), 
        port: 25566, 
        state: LOGIN 
    };
    server.set_registry(&LOGIN_REGISTRY);
    let login_start = LoginStart {
        username: "tenteges".to_string(),
        uuid: None,
    };
    server.put_packet(handshake).await?;
    server.write_packet(login_start).await?;

    match server.next_packet().await? {
        NextPacket::LoginSuccess(p) => println!("loginsuccess"),
        NextPacket::SetCompression(p) => {},
        NextPacket::Disconnect(p) => println!("disconnect"),
        _ => {}
    }

    Ok(server)
}
