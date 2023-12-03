use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, path::Path, fs};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use anyhow::Result;

pub static CONFIG: Lazy<Config> = Lazy::new(|| load_config().unwrap());

pub fn load_config() -> Result<Config> {
    let path = Path::new("config.toml");

    if path.exists() {
        let content = fs::read_to_string(path)?;
        let config = toml::from_str(&content)?;

        return Ok(config);
    }

    let config = Config::default();
    let toml = toml::to_string(&config).unwrap();

    fs::write(path, toml)?;
    Ok(config)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    #[serde(rename = "bind")]
    pub address: SocketAddr,
    pub compression_threshold: i32,
    //pub compression_level: CompressionLvl,
    pub online: bool,
    pub backend_server: SocketAddr,
    pub fallback_server: SocketAddr
}

impl Default for Config {
    fn default() -> Self {
        Self { 
            address: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 25565),
            compression_threshold: 256,
            //compression_level: CompressionLvl::default(),
            online: true,
            backend_server: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 25566),
            fallback_server: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 25567)
        }
    }
}
