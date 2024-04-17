use std::{
    fs, net::{IpAddr, Ipv4Addr, SocketAddr}, path::Path, sync::OnceLock
};

use anyhow::Result;
use libdeflater::CompressionLvl;
use serde::{Deserialize, Serialize, Serializer};

pub fn config() -> &'static Config {
    static CONFIG: OnceLock<Config> = OnceLock::new();
    CONFIG.get_or_init(|| {
        load_config().inspect_err(|err| log::error!("{}", err)).unwrap_or_default()
    })
}

fn load_config() -> Result<Config> {
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
    #[serde(serialize_with = "ser", deserialize_with = "de")]
    pub compression_level: CompressionLvl,
    pub online: bool,
    pub backend_server: SocketAddr,
    pub fallback_server: SocketAddr,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            address: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 25565),
            compression_threshold: 256,
            compression_level: CompressionLvl::default(),
            online: true,
            backend_server: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 25566),
            fallback_server: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 25567),
        }
    }
}

fn ser<S>(level: &CompressionLvl, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_i32(level.into())
}

fn de<'de, D>(deserializer: D) -> Result<CompressionLvl, D::Error>
where
    D: serde::Deserializer<'de>
{
    let level = match i32::deserialize(deserializer) {
        Ok(-1) | Err(_) => return Ok(CompressionLvl::default()),
        Ok(level) => level,
    };

    match CompressionLvl::new(level) {
        Ok(lvl) => Ok(lvl),
        _ => Err(serde::de::Error::custom("invalid compression level (accepted range 1-12)"))
    }
}
