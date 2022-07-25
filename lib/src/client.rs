use std::fs;

use async_trait::async_trait;
use rand::{thread_rng, Rng};
use reqwest::Client;
use rsa::RsaPrivateKey;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{debug, instrument};
use url::Url;

use crate::{
    api_models::{
        AuthyAuthenticatedQueryString, AuthyAuthenticatorTokensQueryString,
        AuthyCheckCurrentDeviceRequest, AuthyCheckDeviceTokensReponse,
        AuthyCheckRegistrationRequest, AuthyCheckRegistrationResponse,
        AuthyCheckRegistrationStatus, AuthyCheckStatusResponse, AuthyCompleteRegistrationRequest,
        AuthyCompleteRegistrationResponse, AuthyListAuthenticatorTokensReponse,
        AuthyListDevicesResponse, AuthyRegisterDeviceRequest, AuthyRegisterDeviceResponse,
        AuthySyncTimeWithServerResponse,
    },
    constants::{API_KEY, DEFAULT_LOCALE, DEVICE_APP_NAME, PRODUCTION_URL},
    error::{MambembeError, Result},
    models::{
        AuthenticatorToken, CheckRegistrationStatus, CheckStatusResponse, Device,
        RegisterDeviceResponse,
    },
    tokens::{calculate_token, get_time},
    utils::{check_api_errors, client_builder, parse_private_key},
};

pub(crate) type AuthyId = u64;

#[derive(Debug, Deserialize, Serialize)]
pub(crate) enum TimeSync {
    Future {
        last_time_checked: u64,
        time_offset: u64,
    },
    Past {
        last_time_checked: u64,
        time_offset: u64,
    },
}

impl TimeSync {
    pub(crate) fn correct_time(&self, time: u64) -> u64 {
        match self {
            TimeSync::Future {
                last_time_checked: _,
                time_offset,
            } => (time + time_offset),
            TimeSync::Past {
                last_time_checked: _,
                time_offset,
            } => (time - time_offset),
        }
    }
}

#[async_trait]
pub trait AuthyClientApi {
    async fn check_user_status(&mut self, phone: &str) -> Result<CheckStatusResponse>;
    async fn register_device(&self) -> Result<RegisterDeviceResponse>;
    async fn check_registration(&self, request_id: &str) -> Result<CheckRegistrationStatus>;
    async fn complete_registration(&mut self, pin: &str) -> Result<()>;
    async fn check_current_device(&self) -> Result<()>;
    async fn check_current_device_keys(&self) -> Result<()>;
    async fn fetch_private_keys(&mut self) -> Result<()>;
    async fn list_devices(&self) -> Result<Vec<Device>>;
    async fn sync_time_with_server(&mut self) -> Result<()>;
    async fn list_authenticator_tokens(&self) -> Result<Vec<AuthenticatorToken>>;
    async fn get_otp_token(&self, authentication_token: &AuthenticatorToken) -> Result<String>;
    fn initialize_authenticator_token(
        &self,
        authentication_token: &mut AuthenticatorToken,
    ) -> Result<()>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthyClient {
    url: Url,
    pub device_name: String,
    pub(crate) signature: String,
    authy_id: Option<AuthyId>,
    request_id: Option<String>,
    device: Option<Device>,
    time_sync: Option<TimeSync>,
    private_key: Option<String>,
    backup_password: String,
    #[serde(skip)]
    parsed_private_key: Option<RsaPrivateKey>,
    #[serde(skip)]
    http_client: Client,
}

impl AuthyClient {
    pub fn new(device_name: &str, backup_password: &str) -> Result<Self> {
        Self::with_url(PRODUCTION_URL, device_name, backup_password)
    }

    pub async fn from_file() -> Result<Self> {
        serde_json::from_slice(&fs::read("device.json").map_err(MambembeError::ConfigFileNotFound)?)
            .map_err(MambembeError::ConfigParsingError)
    }

    #[instrument]
    pub fn with_url(url: &str, device_name: &str, backup_password: &str) -> Result<Self> {
        let mut signature = [0u8; 32];
        thread_rng().fill(&mut signature);
        let signature = signature
            .iter()
            .map(|n| format!("{:x}", n))
            .collect::<String>();

        Ok(Self {
            url: url.parse()?,
            device_name: device_name.to_string(),
            signature,
            backup_password: backup_password.to_string(),
            authy_id: None,
            request_id: None,
            device: None,
            time_sync: None,
            private_key: None,
            parsed_private_key: None,
            http_client: client_builder(),
        })
    }

    /// Not used right now but maybe in the future
    #[allow(dead_code)]
    fn get_private_key(&self) -> Result<&RsaPrivateKey> {
        self.parsed_private_key
            .as_ref()
            .ok_or(MambembeError::PrivateKeyNotFetched)
    }

    fn get_authy_id(&self) -> Result<AuthyId> {
        self.authy_id.ok_or(MambembeError::DeviceNotInitialized)
    }

    fn get_device(&self) -> Result<&Device> {
        self.device
            .as_ref()
            .ok_or(MambembeError::DeviceNotInitialized)
    }
}

#[async_trait]
impl AuthyClientApi for AuthyClient {
    #[instrument]
    async fn check_user_status(&mut self, phone: &str) -> Result<CheckStatusResponse> {
        let response = self
            .http_client
            .get(&format!("{}/users/{}/status", self.url.as_str(), phone))
            .query(&[
                ("api_key", API_KEY),
                ("uuid", "123"),
                ("locale", DEFAULT_LOCALE),
            ])
            .send()
            .await
            .unwrap();

        let response = check_api_errors(response).await?;

        let authy_response: AuthyCheckStatusResponse = response.json().await.unwrap();
        self.authy_id = authy_response.authy_id.clone();

        let output = match authy_response.message.as_str() {
            "new" => CheckStatusResponse::RegisterAccount,
            "active" => CheckStatusResponse::RegisterDevice,
            e => panic!("implement {}", e),
        };
        Ok(output)
    }

    #[instrument]
    async fn register_device(&self) -> Result<RegisterDeviceResponse> {
        let authy_id = self.get_authy_id()?;
        let payload = AuthyRegisterDeviceRequest {
            device_name: self.device_name.clone(),
            signature: self.signature.clone(),
            ..Default::default()
        };

        let response = self
            .http_client
            .post(&format!(
                "{}/users/{}/devices/registration/start",
                self.url.as_str(),
                authy_id
            ))
            .form(&payload)
            .send()
            .await
            .unwrap();

        let response = check_api_errors(response).await?;

        let data: AuthyRegisterDeviceResponse = response.json().await.unwrap();
        Ok(RegisterDeviceResponse::RegistrationPending(data.request_id))
    }

    #[instrument]
    async fn check_registration(&self, request_id: &str) -> Result<CheckRegistrationStatus> {
        let payload = AuthyCheckRegistrationRequest {
            api_key: API_KEY.to_string(),
            locale: DEFAULT_LOCALE.to_string(),
            signature: self.signature.clone(),
        };

        let response = self
            .http_client
            .get(&format!(
                "{}/users/{}/devices/registration/{}/status",
                self.url,
                self.get_authy_id()?,
                request_id
            ))
            .query(&payload)
            .send()
            .await
            .unwrap();
        let response = check_api_errors(response).await?;
        let data: AuthyCheckRegistrationResponse = response.json().await.unwrap();
        Ok(match data.status {
            AuthyCheckRegistrationStatus::Pending => CheckRegistrationStatus::Pending,
            AuthyCheckRegistrationStatus::Accepted => CheckRegistrationStatus::Accepted(
                data.pin.expect("pin not sent when it should have"),
            ),
        })
    }

    #[instrument]
    async fn complete_registration(&mut self, pin: &str) -> Result<()> {
        // I'm assuming this is used for idempotency so this should suffice
        let uuid = format!("{:x}", md5::compute(&pin.as_bytes()));
        let payload = AuthyCompleteRegistrationRequest {
            api_key: API_KEY.to_string(),
            locale: DEFAULT_LOCALE.to_string(),
            pin: pin.to_string(),
            device_app: DEVICE_APP_NAME.to_string(),
            device_name: self.device_name.clone(),
            uuid,
        };
        let response = self
            .http_client
            .post(&format!(
                "{}/users/{}/devices/registration/complete",
                self.url,
                self.get_authy_id()?
            ))
            .form(&payload)
            .send()
            .await
            .unwrap();

        let response = check_api_errors(response).await?;
        let data: AuthyCompleteRegistrationResponse = response.json().await.unwrap();

        self.device = data.device;
        Ok(())
    }

    #[instrument]
    async fn check_current_device(&self) -> Result<()> {
        let device = self
            .device
            .as_ref()
            .ok_or(MambembeError::DeviceNotInitialized)?;

        let payload = AuthyCheckCurrentDeviceRequest {
            api_key: API_KEY.to_string(),
            locale: DEFAULT_LOCALE.to_string(),
            sha: device.hash_secret(),
        };

        let response = self
            .http_client
            .get(&format!(
                "{}/devices/{1}/soft_tokens/{1}/check",
                self.url, device.id
            ))
            .query(&payload)
            .send()
            .await
            .unwrap();

        check_api_errors(response).await?;

        Ok(())
    }

    #[instrument]
    async fn check_current_device_keys(&self) -> Result<()> {
        let device = self.get_device()?;
        let (otp1, otp2, otp3) = device.calculate_tokens(self.time_sync.as_ref());
        let url = format!(
            "{}/users/{}/devices/{}",
            self.url,
            self.get_authy_id()?,
            device.id
        );

        let response = self
            .http_client
            .get(&url)
            .query(&[
                ("api_key", API_KEY),
                ("otp1", &otp1),
                ("otp2", &otp2),
                ("otp3", &otp3),
            ])
            .send()
            .await
            .unwrap();

        let response = check_api_errors(response).await?;
        let _data: AuthyCheckDeviceTokensReponse = response.json().await.unwrap();
        Ok(())
    }

    #[instrument]
    async fn fetch_private_keys(&mut self) -> Result<()> {
        if let Some(key) = self.private_key.as_ref() {
            if self.parsed_private_key.is_none() {
                self.parsed_private_key = Some(parse_private_key(key)?);
            }
            return Ok(());
        }
        let device = self.get_device()?;
        let url = format!("{}/devices/{}/rsa_key", self.url, device.id);
        let response = self
            .http_client
            .get(&url)
            .query(&AuthyAuthenticatedQueryString::with_device(
                device,
                self.time_sync.as_ref(),
            ))
            .send()
            .await
            .unwrap();
        let response = check_api_errors(response).await?;

        let data = response.json::<Value>().await.unwrap();
        debug!("Returned private keys {:?}", data);
        let key = data
            .as_object()
            .map(|o| o.get("private_key"))
            .flatten()
            .map(|k| k.as_str())
            .flatten()
            .expect("failed to get rsa_key");

        self.parsed_private_key = Some(parse_private_key(key)?);
        self.private_key = Some(key.to_string());
        Ok(())
    }

    #[instrument]
    async fn list_devices(&self) -> Result<Vec<Device>> {
        let url = format!("{}/json/users/{}/devices", self.url, self.get_authy_id()?);
        let response = self
            .http_client
            .get(&url)
            .query(&[("api_key", API_KEY)])
            .send()
            .await
            .unwrap();
        let response = check_api_errors(response).await?;
        let data: AuthyListDevicesResponse = response.json().await.unwrap();
        todo!("{:?}", data);
    }

    #[instrument(skip(self))]
    async fn sync_time_with_server(&mut self) -> Result<()> {
        let device = self.get_device()?;
        let url = format!("{}/devices/{}/auth_sync", self.url, device.id);
        let time = get_time(self.time_sync.as_ref());
        let response = self
            .http_client
            .get(&url)
            .query(&AuthyAuthenticatedQueryString::with_device(
                device,
                self.time_sync.as_ref(),
            ))
            .send()
            .await
            .unwrap();

        let response = check_api_errors(response).await?;
        let data: AuthySyncTimeWithServerResponse = response.json().await.unwrap();
        let moving_factor = data.get_moving_factor_in_unix_timestamp();
        let time_sync = if moving_factor < time {
            TimeSync::Past {
                last_time_checked: time,
                time_offset: time - moving_factor,
            }
        } else {
            TimeSync::Future {
                last_time_checked: time,
                time_offset: moving_factor - time,
            }
        };
        self.time_sync = Some(time_sync);

        Ok(())
    }

    #[instrument]
    async fn list_authenticator_tokens(&self) -> Result<Vec<AuthenticatorToken>> {
        let device = self.get_device()?;

        let url = format!(
            "{}/users/{}/authenticator_tokens",
            self.url,
            self.get_authy_id()?
        );
        let response = self
            .http_client
            .get(&url)
            .query(&AuthyAuthenticatorTokensQueryString::with_apps_and_device(
                &[],
                device,
                self.time_sync.as_ref(),
            ))
            .send()
            .await
            .unwrap();
        let response = check_api_errors(response).await?;
        let data: AuthyListAuthenticatorTokensReponse = response.json().await.unwrap();
        Ok(data.authenticator_tokens)
    }

    #[instrument(skip(self, authentication_token), fields(token_name = authentication_token.name.as_str()))]
    async fn get_otp_token(&self, authentication_token: &AuthenticatorToken) -> Result<String> {
        let seed = authentication_token.decrypt_seed()?;
        calculate_token(&seed, authentication_token.digits, self.time_sync.as_ref()).map_err(
            |source| MambembeError::FailedToCalculateToken {
                service_name: authentication_token.name.clone(),
                source,
            },
        )
    }

    fn initialize_authenticator_token(
        &self,
        authentication_token: &mut AuthenticatorToken,
    ) -> Result<()> {
        authentication_token.initialize_token(&self.backup_password);
        Ok(())
    }
}
