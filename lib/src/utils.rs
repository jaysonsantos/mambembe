use data_encoding::BASE64;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use rsa::{pkcs1::DecodeRsaPrivateKey, RsaPrivateKey};
use tracing::debug;

use crate::api_models::AuthyApiError;
pub use crate::error::{MambembeError, Result};

const DAMAGED_TOKEN_ERROR: &str = "60043";

pub(crate) fn client_builder() -> Client {
    let mut headers = HeaderMap::new();
    headers.insert(
        "X-User-Agent",
        HeaderValue::from_str(&format!("Mambembe v{}", env!("CARGO_PKG_VERSION")))
            .expect("invalid caracters on header value"),
    );
    headers.insert(
        "User-Agent",
        HeaderValue::from_str(&format!(
            "Mambembe/{} (+https://github.com/jaysonsantos/mambembe)",
            env!("CARGO_PKG_VERSION")
        ))
        .expect("invalid caracters on header value"),
    );

    Client::builder()
        .gzip(true)
        .default_headers(headers)
        .build()
        .unwrap()
}

pub(crate) async fn check_api_errors(response: reqwest::Response) -> Result<reqwest::Response> {
    match response.error_for_status_ref() {
        Err(source) => {
            let body = match response.text().await {
                Ok(body) => body,
                Err(err) => {
                    let body = format!("failed to decode body: {:?}", err);
                    return Err(MambembeError::ApiError { body, source });
                }
            };
            let parsed_body = serde_json::from_str::<AuthyApiError>(&body);
            match parsed_body {
                Err(err) => {
                    debug!("failed to parse error body {:?}", err);
                    return Err(MambembeError::ApiError { body, source });
                }
                Ok(parsed_body) => match parsed_body.error_code.as_str() {
                    DAMAGED_TOKEN_ERROR => Err(MambembeError::DamagedToken),
                    _ => Err(MambembeError::ApiError { body, source }),
                },
            }
        }
        _ => Ok(response),
    }
}

pub(crate) fn parse_private_key(key: &str) -> Result<RsaPrivateKey> {
    let key: String = key.lines().filter(|l| !l.starts_with("-")).collect();
    let decoded = BASE64.decode(key.as_bytes()).unwrap();

    let private_key = RsaPrivateKey::from_pkcs1_der(&decoded)?;
    Ok(private_key)
}
