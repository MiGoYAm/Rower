use thiserror::Error;

use crate::component::Component;

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("disconnected")]
    Disconnected(Component),
    #[error(transparent)]
    Other(#[from] anyhow::Error)
} 