use std::{io, result};

use data_encoding::DecodeError;
use thiserror::Error;

pub type Result<T> = result::Result<T, MambembeError>;
pub(crate) type InternalResult<T> = result::Result<T, InternalError>;

#[derive(Debug, Error)]
pub enum MambembeError {
    #[error("invalid url")]
    InvalidUrl(#[from] url::ParseError),
    #[error("device not initialized")]
    DeviceNotInitialized,
    #[error("private key not fetched")]
    PrivateKeyNotFetched,
    #[error("config file not found")]
    ConfigFileNotFound(#[from] io::Error),
    #[error("failed to parse config")]
    ConfigParsingError(#[from] serde_json::Error),
    #[error("damaged token halp {0}")]
    DamagedToken(String),
    #[error("api error {body}")]
    ApiError {
        body: String,
        source: reqwest::Error,
    },
    #[error("private key error")]
    PrivateKeyError(#[from] rsa::pkcs1::Error),
    #[error("token not initialized")]
    AuthenticatorTokenNotInitialized,
    #[error("failed to calculate token for service {service_name:?}")]
    FailedToCalculateToken {
        service_name: String,
        source: InternalError,
    },
    #[error("failed to decrypt seed for service {service_name:?}")]
    FailedToDecryptSeed {
        service_name: String,
        source: InternalError,
    },
}

#[derive(Debug, Error)]
pub enum InternalError {
    /// Never goes to clients
    #[error("decode failed")]
    DecodeFailed(#[from] DecodeError),
    #[error("decryption error unpad error")]
    DecryptionError,
}
