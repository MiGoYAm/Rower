use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use once_cell::sync::Lazy;
use serde::Deserialize;

pub static CONFIG: Lazy<Config> = Lazy::new(|| envy::prefixed("ROWER_").from_env().unwrap());

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default = "default_address")]
    pub address: SocketAddr,
    #[serde(default = "default_threshold")]
    pub threshold: i32,
    #[serde(default = "default_online_mode")]
    pub online: bool,
    #[serde(default = "default_backend_server")]
    pub backend_server: SocketAddr
}

fn default_address() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 25565)
}

fn default_threshold() -> i32 {
    256
}

fn default_online_mode() -> bool {
    true
}

fn default_backend_server() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 25566)
}
