use lazy_static::lazy_static;
use mambembe_lib::{
    client::AuthyClientApi,
    models::{CheckRegistrationStatus, CheckStatusResponse, RegisterDeviceResponse},
    AuthyClient,
};
use mambembe_stub_server::start_wiremock;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::time::sleep;

lazy_static! {
    static ref CLIENT_CONFIG: Value = json!({
        "device_name": "test",
        "signature": "abcde",
        "authy_id": 1234,
        "backup_password": "abc",
        "device": {
            "id": 12334,
            "secret_seed": "1bcc2b0a43e94a90916a04079190af40",
        }
    });
}

// #[tokio::test]
// async fn vla() {
//     let client: AuthyClient = serde_json::from_str(include_str!("device.json")).unwrap();
//     client.check_current_device().await.unwrap();
// }

fn get_test_client(wiremock_url: &str) -> AuthyClient {
    let mut client_config = CLIENT_CONFIG.clone();
    client_config.as_object_mut().unwrap().insert(
        "url".to_string(),
        Value::String(format!("{}/json", wiremock_url)),
    );

    serde_json::from_value(client_config).unwrap()
}

#[cfg(docker)]
#[tokio::test]
async fn list_authenticator_tokens() {
    let url = start_wiremock().await.unwrap();
    let client = get_test_client(&url);
    let tokens = client.list_authenticator_tokens().await.unwrap();
    assert_eq!(tokens.len(), 2);
    let lastpass = &tokens[0];
    assert_eq!(lastpass.name, "LastPass");
}

#[cfg(docker)]
#[tokio::test]
async fn check_current_device() {
    let url = start_wiremock().await.unwrap();
    let client = get_test_client(&url);
    client.check_current_device_keys().await.unwrap();
}

#[cfg(docker)]
#[tokio::test]
async fn check_current_device_keys() {
    let url = start_wiremock().await.unwrap();
    let client = get_test_client(&url);
    client.check_current_device().await.unwrap();
}

#[cfg(docker)]
#[tokio::test]
async fn register_flow() {
    let url = start_wiremock().await.unwrap();

    let mut client =
        AuthyClient::with_url(&format!("{}/json", url), "test-device", "1234").unwrap();
    assert_eq!(
        CheckStatusResponse::RegisterDevice,
        client.check_user_status("123456").await.unwrap()
    );
    let response = client.register_device().await.unwrap();
    let RegisterDeviceResponse::RegistrationPending(request_id) = response;

    let response = client.check_registration(&request_id).await.unwrap();
    assert_eq!(CheckRegistrationStatus::Pending, response);
    let pin = loop {
        let response = client.check_registration(&request_id).await.unwrap();
        match response {
            CheckRegistrationStatus::Accepted(pin) => break pin,
            CheckRegistrationStatus::Pending => sleep(Duration::from_millis(10)).await,
        }
    };
    let _ = client.complete_registration(&pin).await.unwrap();
}
