/// This mimics keyring's api but uses security command to interface with keychain.
use std::{io, process, result};

use thiserror::Error;

type Result<T> = result::Result<T, KeyringError>;

#[derive(Debug, Error)]
pub enum KeyringError {
    #[error("no password found")]
    NoEntry,
    #[error("could not determine apps directory")]
    AppDirectoryNotFound,
    #[error("io error")]
    IoError(#[from] io::Error),
}

pub struct Keyring {
    service_name: String,
    username: String,
}

impl Keyring {
    pub fn new(service_name: &str, username: &str) -> Self {
        let service_name = service_name.to_string();
        let username = username.to_string();
        Self {
            service_name,
            username,
        }
    }

    pub fn get_password(&self) -> Result<String> {
        let command = process::Command::new("security")
            .arg("find-generic-password")
            .arg("-a")
            .arg(&self.username)
            .arg("-s")
            .arg(&self.service_name)
            .arg("-w")
            .output()?;
        if let Some(stdout) = command.stdout.strip_suffix(&[b'\n']) {
            return Ok(
                String::from_utf8_lossy(&hex::decode(stdout).expect("invalid hex")).to_string(),
            );
        }
        Err(KeyringError::NoEntry)
    }

    pub fn set_password(&self, password: &str) -> Result<()> {
        process::Command::new("security")
            .arg("add-generic-password")
            .arg("-a")
            .arg(&self.username)
            .arg("-s")
            .arg(&self.service_name)
            .arg("-w")
            .arg(password)
            .output()?;
        Ok(())
    }
}
