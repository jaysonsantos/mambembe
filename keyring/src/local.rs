/// This mimics keyring's api but just saves files locally.
use std::{fs, io, path::PathBuf, result};

use directories::ProjectDirs;
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
        let file_name = self.get_storage_file()?;
        fs::read_to_string(&file_name).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => KeyringError::NoEntry,
            _ => e.into(),
        })
    }

    pub fn set_password(&self, password: &str) -> Result<()> {
        let file_name = self.get_storage_file()?;
        fs::create_dir_all(&file_name.parent().unwrap())?;
        Ok(fs::write(file_name, password.as_bytes())?)
    }

    fn get_storage_file(&self) -> Result<PathBuf> {
        Ok(self
            .get_project_directory()?
            .config_dir()
            .join(&self.username))
    }

    fn get_project_directory(&self) -> Result<ProjectDirs> {
        ProjectDirs::from("com", "Jayson Reis", &self.service_name)
            .ok_or(KeyringError::AppDirectoryNotFound)
    }
}
