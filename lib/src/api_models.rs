use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    client::{AuthyId, TimeSync},
    constants::{API_KEY, DEFAULT_LOCALE, DEVICE_APP_NAME},
    models::{AuthenticatorToken, Device},
};

#[derive(Debug, Deserialize)]
pub(crate) struct AuthyApiError {
    pub error_code: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Default, PartialEq)]
pub(crate) struct AuthyCheckStatusResponse {
    force_ott: bool,
    pub message: String,
    devices_count: Option<usize>,
    pub authy_id: Option<AuthyId>,
    success: bool,
}

#[derive(Debug, Serialize, PartialEq)]
pub(crate) struct AuthyRegisterDeviceRequest {
    pub(crate) api_key: String,
    pub(crate) locale: String,
    pub(crate) via: String,
    pub signature: String,
    pub device_app: String,
    pub(crate) device_name: String,
}
#[derive(Debug, Deserialize, PartialEq, Default)]
pub(crate) struct AuthyRegisterDeviceResponse {
    message: String,
    pub request_id: String,
    approval_pin: usize,
    provider: String,
    success: bool,
}

impl Default for AuthyRegisterDeviceRequest {
    fn default() -> Self {
        Self {
            api_key: API_KEY.to_string(),
            locale: DEFAULT_LOCALE.to_string(),
            via: "push".to_string(),
            signature: "".to_string(),
            device_app: DEVICE_APP_NAME.to_string(),
            device_name: "".to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct AuthyCheckRegistrationRequest {
    pub api_key: String,
    pub locale: String,
    pub signature: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub(crate) struct AuthyCheckRegistrationResponse {
    pub message: Value,
    pub status: AuthyCheckRegistrationStatus,
    pub pin: Option<String>,
    pub success: bool,
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum AuthyCheckRegistrationStatus {
    Pending,
    Accepted,
}

#[derive(Debug, Serialize)]
pub(crate) struct AuthyCompleteRegistrationRequest {
    pub api_key: String,
    pub locale: String,
    pub pin: String,
    pub device_app: String,
    pub device_name: String,
    pub uuid: String,
}
#[derive(Debug, Deserialize)]
pub(crate) struct AuthyCompleteRegistrationResponse {
    authy_id: u64,
    pub device: Option<Device>,
}
#[derive(Debug, Serialize)]
pub(crate) struct AuthyCheckCurrentDeviceRequest {
    pub api_key: String,
    pub locale: String,
    pub sha: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuthyListDevicesResponse {
    devices: Vec<AuthyDevice>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuthyDevice {
    _id: String,
    api_key: String,
    city: String,
    country: String,
    created_at: String,
    device_app: String,
    device_type: String,
    name: String,
    user_agent: String,
    registered: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuthyListAuthenticatorTokensReponse {
    pub authenticator_tokens: Vec<AuthenticatorToken>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuthyCheckDeviceTokensReponse {
    cellphone: String,
    country_code: u8,
    success: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct AuthyAuthenticatedQueryString {
    api_key: String,
    device_id: String,
    otp1: String,
    otp2: String,
    otp3: String,
}

impl AuthyAuthenticatedQueryString {
    pub(crate) fn with_device(device: &Device, time_sync: Option<&TimeSync>) -> Self {
        let (otp1, otp2, otp3) = device.calculate_tokens(time_sync);
        Self {
            api_key: API_KEY.to_string(),
            device_id: device.id.to_string(),
            otp1,
            otp2,
            otp3,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct AuthyAuthenticatorTokensQueryString {
    apps: String,
    locale: String,
    #[serde(flatten)]
    authentication: AuthyAuthenticatedQueryString,
}

impl AuthyAuthenticatorTokensQueryString {
    pub fn with_apps_and_device(
        apps: &[String],
        device: &Device,
        time_sync: Option<&TimeSync>,
    ) -> Self {
        Self {
            apps: apps.join(","),
            locale: DEFAULT_LOCALE.to_string(),
            authentication: AuthyAuthenticatedQueryString::with_device(device, time_sync),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuthySyncTimeWithServerResponse {
    moving_factor: String,
}

impl AuthySyncTimeWithServerResponse {
    pub fn get_moving_factor_in_unix_timestamp(&self) -> u64 {
        // Right pad with zeroes on the right as the sent value is not really a valid
        // timestamp
        let padded = format!("{:0<10}", self.moving_factor);
        padded.parse().expect("failed to parse moving factor")
    }
}
