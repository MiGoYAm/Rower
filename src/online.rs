use std::sync::LazyLock;

use anyhow::Result;
use md5::{Digest, Md5};
use num_bigint::BigInt;
use openssl::{encrypt::Decrypter, pkey::{PKey, Private}};
use openssl::rsa::Rsa;
use serde::Deserialize;
use sha1::Sha1;
use uuid::Uuid;

pub struct Keys {
    pub pair_key: PKey<Private>,
    pub public_key: Vec<u8>,
}

pub static RSA_KEYS: LazyLock<Keys> = LazyLock::new(|| {
    let pkey = PKey::from_rsa(Rsa::generate(1024).unwrap()).unwrap();
    let public_key = pkey.public_key_to_pem().unwrap();
    Keys {
        pair_key: pkey,
        public_key,
    }
});

#[derive(Deserialize)]
pub struct GameProfile {
    pub id: Uuid,
    pub name: String,
    pub properties: Vec<Property>,
}

#[derive(Deserialize)]
pub struct Property {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

pub fn decrypt(decrypter: &mut Decrypter, data: &[u8]) -> Result<Vec<u8>> {
    let mut to = vec![0; decrypter.decrypt_len(data)?];
    let len = decrypter.decrypt(data, &mut to)?;
    to.truncate(len);
    Ok(to)
}

pub fn generate_server_id(shared_secret: &[u8], public_key: &[u8]) -> Result<String> {
    let hash = Sha1::new()
        .chain_update(shared_secret)
        .chain_update(public_key)
        .finalize();
    let str = BigInt::from_signed_bytes_be(&hash).to_str_radix(16);
    Ok(str)
}

pub fn generate_offline_uuid(username: &String) -> Uuid {
    let hash = Md5::new_with_prefix(b"OfflinePlayer:")
        .chain_update(username.as_bytes())
        .finalize();

    uuid::Builder::from_md5_bytes(hash.into()).into_uuid()
}
