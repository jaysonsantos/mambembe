#[cfg(feature = "without-keyring")]
mod local;

use std::result;

#[cfg(feature = "with-keyring")]
use keyring::{Entry as Keyring, Error as KeyringError};
use lazy_static::lazy_static;
use mambembe_lib::{models::AuthenticatorToken, AuthyClient};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{from_str, to_string_pretty};
use thiserror::Error;

#[cfg(feature = "without-keyring")]
use crate::local::{Keyring, KeyringError};

const SERVICE_NAME: &str = "mambembe";
lazy_static! {
    static ref DEVICES: Keyring = Keyring::new(SERVICE_NAME, "devices.json");
    static ref TOKENS: Keyring = Keyring::new(SERVICE_NAME, "tokens.json");
}

type Result<T> = result::Result<T, MambembeKeyringError>;

#[derive(Debug, Error)]
pub enum MambembeKeyringError {
    #[error("password not stored in the keyring yet")]
    NoPasswordFound,
    #[error("deserialization error")]
    DeserializationError(#[from] serde_json::Error),
    #[error("unknown keyring backend error")]
    UnknownBackendError(#[from] KeyringError),
}

pub trait Data<T> {
    fn get_keyring() -> &'static Keyring;
}

impl<T> Data<T> for AuthyClient {
    fn get_keyring() -> &'static Keyring {
        &DEVICES
    }
}

impl<T> Data<T> for Vec<AuthenticatorToken> {
    fn get_keyring() -> &'static Keyring {
        &TOKENS
    }
}

pub fn get<T>() -> Result<T>
where
    T: DeserializeOwned + Data<T>,
{
    let data = match T::get_keyring().get_password() {
        Ok(data) => data,
        Err(KeyringError::NoEntry) => return Err(MambembeKeyringError::NoPasswordFound),
        Err(err) => return Err(MambembeKeyringError::UnknownBackendError(err)),
    };
    Ok(from_str(&data)?)
}

pub fn set<T>(data: &T) -> Result<()>
where
    T: Serialize + Data<T>,
{
    let data = to_string_pretty(data)?;
    T::get_keyring()
        .set_password(&data)
        .map_err(MambembeKeyringError::UnknownBackendError)?;
    Ok(())
}
