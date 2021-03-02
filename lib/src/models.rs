use data_encoding::HEXLOWER;
use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::{
    client::TimeSync, crypto::decrypt_data, error::Result, password::derive_key,
    tokens::calculate_future_tokens, MambembeError,
};

pub type Pin = String;
pub type RequestId = String;

#[derive(Debug, Serialize, Deserialize)]
pub struct Device {
    pub id: u64,
    secret_seed: String,
}

#[derive(Debug, PartialEq)]
pub enum CheckStatusResponse {
    RegisterDevice,
    RegisterAccount,
}

#[derive(Debug, PartialEq)]
pub enum RegisterDeviceResponse {
    RegistrationPending(RequestId),
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CheckRegistrationStatus {
    Pending,
    Accepted(Pin),
}

impl Device {
    pub(crate) fn hash_secret(&self) -> String {
        format!("{:x}", sha2::Sha256::digest(&self.secret_seed.as_bytes()))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthenticatorToken {
    pub account_type: String,
    pub digits: usize,
    pub(crate) encrypted_seed: String,
    pub name: String,
    original_name: Option<String>,
    password_timestamp: u64,
    salt: String,
    unique_id: String,
    derived_key: Option<Vec<u8>>,
}

impl Device {
    pub(crate) fn calculate_tokens(
        &self,
        time_sync: Option<&TimeSync>,
    ) -> (String, String, String) {
        let seed = HEXLOWER
            .decode(&self.secret_seed.as_bytes())
            .expect("invalid secret");

        calculate_future_tokens(&seed, time_sync)
    }
}

impl AuthenticatorToken {
    pub fn initialize_token(&mut self, password: &str) {
        if self.derived_key.is_some() {
            return;
        }
        self.derived_key = Some(derive_key(password, &self.salt));
    }

    pub fn decrypt_seed(&self) -> Result<Vec<u8>> {
        let derived_key = self
            .derived_key
            .as_ref()
            .ok_or(MambembeError::AuthenticatorTokenNotInitialized)?;

        let data = decrypt_data(derived_key, &self.encrypted_seed).map_err(|source| {
            MambembeError::FailedtoDecryptSeed {
                service_name: self.name.clone(),
                source,
            }
        })?;

        Ok(data)
    }
}
