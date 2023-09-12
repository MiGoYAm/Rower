use thiserror::Error;

use crate::component::Component;

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("server disconnected")]
    ServerDisconnected(Component),
    #[error(transparent)]
    Other(#[from] anyhow::Error)
}