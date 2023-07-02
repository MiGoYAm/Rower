use std::net::{SocketAddr, IpAddr, Ipv4Addr};

pub const ADDRESS: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 25565);

pub const THRESHOLD: i32 = 256;
pub const ONLINE: bool = false;

pub const SERVERS: [Server; 1] = [
    Server {
        name: "main",
        address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 25566)
    }
];

pub struct Server {
    pub name: &'static str,
    pub address: SocketAddr
}
