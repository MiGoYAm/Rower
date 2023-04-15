use std::io::Cursor;

use base64::{engine::general_purpose, Engine};
use image::{ImageError, error::{LimitError, LimitErrorKind}, image_dimensions, ImageOutputFormat};
use image::io::Reader as ImageReader;
use log::warn;
use once_cell::sync::Lazy;

use crate::{protocol::packet::status::{Status, Version, Players, Motd}, component::TextComponent};

//type Handler = fn(Connection) -> Result<(), Box<dyn Error>>;

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
        description: Motd::Component(TextComponent::new("azz".to_string())), 
        favicon: optional_favicon(),
    };
    serde_json::to_vec(&status).unwrap()
});

/*
pub static STATES: Lazy<Arc<RwLock<Vec<u8>>>> = Lazy::new(|| {
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
        description: Motd::Component(TextComponent::new("azz".to_string())), 
        favicon: optional_favicon(),
    };
    Arc::new(RwLock::new(serde_json::to_vec(&status).unwrap()))
});

pub fn update() {
    let pointer = STATES.clone();
    let writer = (pointer.write()).writer();
    let status = Status {
        version: Version {
            name: "1.19.4",
            protocol: 762,
        },
        players: Players {
            online: 3, 
            max: 16, 
            sample: vec![],
        },
        description: Motd::Component(TextComponent::new("azz".to_string())), 
        favicon: optional_favicon(),
    };
    serde_json::to_writer(writer, &status);
}
*/

fn optional_favicon() -> Option<String>{
    match read_favicon() {
        Ok(x) => Some(x),
        Err(e) => {
            warn!("{}", e);
            None
        },
    }
}

fn read_favicon() -> Result<String, ImageError> {
    const PATH: &str = "server-icon.png";

    let dimensions = image_dimensions(PATH)?;
    if dimensions != (64, 64) {
        return Err(ImageError::Limits(LimitError::from_kind(LimitErrorKind::DimensionError)));
    }

    let file_image = ImageReader::open(PATH)?;
    let mut buffer = Vec::with_capacity(4096);

    file_image.decode()?.write_to(&mut Cursor::new(&mut buffer), ImageOutputFormat::Png)?;
    let favicon = general_purpose::STANDARD_NO_PAD.encode(buffer);

    Ok(format!("{}{}", "data:image/png;base64,", favicon))
}