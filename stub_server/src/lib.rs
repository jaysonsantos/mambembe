use std::{
    process::{Output, Stdio},
    sync::Arc,
    time::Duration,
};

use color_eyre::{
    eyre::{eyre, WrapErr},
    Help, Result, SectionExt,
};
use lazy_static::lazy_static;
use tokio::{process::Command, sync::Mutex, time::sleep};
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
        let host = host.trim();
        let url = if host.starts_with(':') {
            format!("http://localhost{}", host.trim())
        } else {
            format!("http://{}", host.trim())
        };

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
