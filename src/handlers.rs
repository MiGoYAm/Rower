use std::io::Cursor;

use base64::{engine::general_purpose, Engine};
use image::io::Reader as ImageReader;
use image::{
    error::{LimitError, LimitErrorKind},
    image_dimensions, ImageError, ImageOutputFormat,
};
use log::warn;
use once_cell::sync::Lazy;

use crate::{
    component::Component,
    protocol::packet::status::{Motd, Players, Status, Version},
};

pub static STATES: Lazy<Vec<u8>> = Lazy::new(|| {
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
        description: Motd::Component(Component::text_str("azz")),
        favicon: optional_favicon(),
    };
    serde_json::to_vec(&status).unwrap()
});

fn optional_favicon() -> Option<String> {
    match read_favicon() {
        Ok(x) => Some(x),
        Err(e) => {
            warn!("{}", e);
            None
        }
    }
}

fn read_favicon() -> anyhow::Result<String> {
    const PATH: &str = "server-icon.png";

    let dimensions = image_dimensions(PATH)?;
    if dimensions != (64, 64) {
        return Err(ImageError::Limits(LimitError::from_kind(LimitErrorKind::DimensionError)).into());
    }

    let file_image = ImageReader::open(PATH)?;
    let mut buffer = Vec::with_capacity(4096);

    file_image.decode()?.write_to(&mut Cursor::new(&mut buffer), ImageOutputFormat::Png)?;
    let favicon = general_purpose::STANDARD_NO_PAD.encode(buffer);

    Ok(format!("{}{}", "data:image/png;base64,", favicon))
}
