use std::sync::Arc;

use rand::{thread_rng, Rng};
use serde::Deserialize;
use tide::{convert::json, prelude::Listener, Response, StatusCode};
use tide::{listener::ToListener, Request as TideRequest};
use tokio::{
    sync::{oneshot, Mutex},
    task::{self, JoinHandle},
};

type Request = TideRequest<Arc<Mutex<State>>>;

struct State {
    registration_accepted: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            registration_accepted: false,
        }
    }
}

pub struct StubServer {
    pub url: String,
    handle: JoinHandle<()>,
}

impl Drop for StubServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

pub async fn start_stub_server() -> StubServer {
    let (tx, rx) = oneshot::channel::<()>();
    let mut rng = thread_rng();
    let port: u16 = rng.gen_range(12000..20000);
    let url = format!("http://127.0.0.1:{}/json", port);
    let mut app = tide::with_state(Arc::new(Mutex::new(State::default())));
    app.at("/json/users/:phone/status").get(check_device_status);
    app.at("/json/users/:authy_id/devices/registration/start")
        .post(registration_start);
    app.at("/json/users/:authy_id/devices/registration/:request_id/status")
        .get(registration_status);
    app.at("/json/users/:authy_id/devices/registration/complete")
        .post(registration_complete);
    app.at("/json/devices/:device_id/soft_tokens/:device_id/check")
        .get(check_device_status);
    app.at("/json/users/:authy_id/devices/:device_id")
        .get(check_device_tokens);
    app.at("/json/users/:authy_id/authenticator_tokens")
        .get(list_authenticator_tokens);

    let mut listener = format!("127.0.0.1:{}", port).to_listener().unwrap();
    let handle = task::spawn(async move {
        listener.bind(app).await.unwrap();
        tx.send(()).unwrap();
        println!("bound");
        listener.accept().await.unwrap();
    });

    rx.await.unwrap();
    StubServer { url, handle }
}

async fn check_device_status(_: Request) -> tide::Result {
    Ok(json!({
        "authy_id": 12345,
        "devices_count": 1,
        "force_ott": true,
        "message": "active",
        "success": true
    })
    .into())
}

async fn registration_start(_request: Request) -> tide::Result {
    Ok(json!({
        "approval_pin": 1,
        "message": "A request was sent to your other devices.",
        "provider": "push",
        "request_id": "603a4d9e613cafeac8e36234d",
        "success": true
    })
    .into())
}

async fn registration_status(request: Request) -> tide::Result {
    let state = request.state().clone();
    let mut state = state.lock().await;
    if state.registration_accepted {
        Ok(json!({
            "message": {
                "request_status": "Request Status."
            },
            "pin": "12345",
            "status": "accepted",
            "success": true
        })
        .into())
    } else {
        state.registration_accepted = true;
        Ok(json!({
            "message": {
                "request_status": "Request Status."
            },
            "status": "pending",
            "success": true
        })
        .into())
    }
}

async fn registration_complete(request: Request) -> tide::Result {
    Ok(json!({
        "authy_id": request.param("authy_id").unwrap().parse::<u64>().unwrap(),
        "device": {
            "api_key": "not important here",
            "id": 321321,
            "reinstall": false,
            "secret_seed": "48bebacafe22334beba47dcafe37252a"
        }
    })
    .into())
}

async fn list_authenticator_tokens(_request: Request) -> tide::Result {
    Ok(json!({
    "authenticator_tokens": [
        {
            "account_type": "lastpass",
            "digits": 6,
            "encrypted_seed": "ONQWIZTTMFSHGYLEMFSAU===",
            "issuer": null,
            "logo": null,
            "name": "LastPass",
            "original_name": "LastPass",
            "password_timestamp": 1435323862,
            "salt": "dsdsad",
            "unique_id": "3213213"
        },
        {
            "account_type": "digitalocean",
            "digits": 6,
            "encrypted_seed": "ONQWIZTTMFSHGYLEMFSAU",
            "issuer": null,
            "logo": null,
            "name": "Digital Ocean",
            "original_name": "DigitalOcean",
            "password_timestamp": 1435323862,
            "salt": "dsadsad",
            "unique_id": "3213213"
        }
        ]
    })
    .into())
}

#[allow(dead_code)]
async fn check_current_device(_request: Request) -> tide::Result {
    Ok(json!({
        "message": "Token is correct.",
        "success": true
    })
    .into())
}

#[derive(Debug, Deserialize)]
struct OtpTokens {
    otp1: u32,
    otp2: u32,
    otp3: u32,
}

async fn check_device_tokens(_request: Request) -> tide::Result {
    // TODO: validate tokens
    let _tokens: OtpTokens = match _request.query() {
        Ok(tokens) => tokens,
        Err(err) => {
            let mut response = Response::new(StatusCode::BadRequest);
            response.set_body(json!({
                "message": "otp tokens not detected",
                "error": err.to_string()
            }));

            return Ok(response);
        }
    };
    Ok(json!({
        "cellphone": "17172720",
        "country_code": 49,
        "email": "fakeuser@gmail.com",
        "multidevice_enabled": true,
        "multidevices_enabled": true,
        "success": true,
    })
    .into())
}

use std::{
    process::{Output, Stdio},
    time::Duration,
};

use color_eyre::{
    eyre::{eyre, WrapErr},
    Help, Result, SectionExt,
};
use lazy_static::lazy_static;
use tokio::{process::Command, time::sleep};
use tracing::trace;

lazy_static! {
    static ref WIREMOCK: Arc<Mutex<Option<WiremockRunner>>> = Arc::new(Mutex::new(None));
}

struct WiremockRunner {
    url: String,
}

fn compose() -> Command {
    let mut command = Command::new("docker-compose");
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    command
}

impl WiremockRunner {
    pub async fn new() -> Result<Self> {
        trace!("Starting with docker");

        check_process(
            &compose()
                .args(&["up", "-d"])
                .output()
                .await
                .wrap_err("error to bring wiremock up")?,
        )?;

        let host = check_process(
            &compose()
                .args(&["port", "wiremock", "8080"])
                .output()
                .await
                .wrap_err("failed to get wiremock port")?,
        )?;
        let url = format!("http://{}", host.trim());

        Ok(Self { url })
    }

    async fn wait_for_server(&self) -> Result<()> {
        let mut last_error = None;
        for _ in 0..30 {
            match reqwest::get(&format!("{}/__admin", &self.url)).await {
                Ok(_) => {
                    return Ok(());
                }
                Err(err) => last_error = Some(err),
            }

            sleep(Duration::from_millis(300)).await;
        }
        Err(eyre!(last_error.unwrap()).wrap_err("wiremock never went up"))
    }
}

fn check_process(output: &Output) -> Result<String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        return Ok(stdout.to_string());
    }

    let err = eyre!("failed to run command")
        .with_section(move || format!("Exit code: {:?}", output.status.code()))
        .with_section(move || stdout.trim().to_string().header("Stdout:"))
        .with_section(move || stderr.trim().to_string().header("Stderr:"));

    Err(err)
}

pub async fn start_wiremock() -> Result<String> {
    let wiremock_guard = WIREMOCK.lock().await;
    if let Some(wiremock) = &*wiremock_guard {
        return Ok(wiremock.url.clone());
    }

    let wiremock = WiremockRunner::new().await?;
    let url = wiremock.url.clone();
    wiremock.wait_for_server().await?;

    Ok(url)
}
