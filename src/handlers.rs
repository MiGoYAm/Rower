use std::io::Cursor;
use std::net::SocketAddr;
use std::sync::OnceLock;

use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use image::io::Reader as ImageReader;
use image::{
    error::{LimitError, LimitErrorKind},
    image_dimensions, ImageError, ImageFormat,
};
use log::error;

use crate::config::config;
use crate::{
    component::Component,
    protocol::packet::status::{Motd, Players, Status, Version},
};

pub fn status() -> &'static Vec<u8> {
    static STATUS: OnceLock<Vec<u8>> = OnceLock::new();
    STATUS.get_or_try_init(create_status).unwrap()
}

pub fn create_status() -> Result<Vec<u8>> {
    let status = Status {
        version: Version {
            name: "1.19.4",
            protocol: 762,
        },
        players: Players {
            online: 2,
            max: 16,
            sample: vec![],
        },
        description: Motd::Component(Component::text("azz")),
        favicon: read_favicon().inspect_err(|err| error!("{}", err)).ok(),
    };
    Ok(serde_json::to_vec(&status)?)
}

fn read_favicon() -> Result<String> {
    const PATH: &str = "server-icon.png";

    let dimensions = image_dimensions(PATH)?;
    if dimensions != (64, 64) {
        return Err(
            ImageError::Limits(LimitError::from_kind(LimitErrorKind::DimensionError)).into(),
        );
    }

    let file_image = ImageReader::open(PATH)?;
    let mut buffer = Vec::with_capacity(4096);

    file_image
        .decode()?
        .write_to(&mut Cursor::new(&mut buffer), ImageFormat::Png)?;
    let favicon = general_purpose::STANDARD_NO_PAD.encode(buffer);

    Ok(format!("data:image/png;base64,{}", favicon))
}

pub fn get_initial_server() -> SocketAddr {
    config().backend_server
}
