use std::result;

use keyring::{Keyring, KeyringError};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{from_str, to_string_pretty};
use thiserror::Error;

use mambembe_lib::{models::AuthenticatorToken, AuthyClient};

const SERVICE_NAME: &str = "mambembe";
const DEVICES: Keyring = Keyring::new(SERVICE_NAME, "devices.json");
const TOKENS: Keyring = Keyring::new(SERVICE_NAME, "tokens.json");

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
    fn get_keyring() -> &'static Keyring<'static>;
}

impl<T> Data<T> for AuthyClient {
    fn get_keyring() -> &'static Keyring<'static> {
        &DEVICES
    }
}

impl<T> Data<T> for Vec<AuthenticatorToken> {
    fn get_keyring() -> &'static Keyring<'static> {
        &TOKENS
    }
}

pub fn get<T>() -> Result<T>
where
    T: DeserializeOwned + Data<T>,
{
    let data = match T::get_keyring().get_password() {
        Ok(data) => data,
        Err(KeyringError::NoPasswordFound) => return Err(MambembeKeyringError::NoPasswordFound),
        Err(err) => return Err(err.into()),
    };
    Ok(from_str(&data)?)
}

pub fn set<T>(data: &T) -> Result<()>
where
    T: Serialize + Data<T>,
{
    let data = to_string_pretty(data)?;
    T::get_keyring().set_password(&data)?;
    Ok(())
}
