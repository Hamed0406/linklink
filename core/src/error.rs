use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("STUN error: {0}")]
    Stun(String),

    #[error("Invalid interface name: {0}")]
    InvalidInterfaceName(String),

    #[error("WireGuard error: {0}")]
    WireGuard(String),

    #[error("Invite error: {0}")]
    Invite(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Network timeout")]
    Timeout,

    #[error("Command failed (exit {code}): {stderr}")]
    CommandFailed { code: i32, stderr: String },
}

pub type Result<T> = std::result::Result<T, Error>;
