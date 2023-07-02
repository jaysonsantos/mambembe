use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Mambembe(#[from] mambembe_lib::MambembeError),
    #[error(transparent)]
    KeyRing(#[from] mambembe_keyring::MambembeKeyringError),
}
