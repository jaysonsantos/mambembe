use std::{process::exit, time::Duration};

use color_eyre::{
    eyre::{eyre, Context},
    Report, Result,
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use structopt::StructOpt;
use tokio::{
    io,
    io::{AsyncBufReadExt, BufReader},
    time::sleep,
};
use tracing::{info, instrument};

use mambembe_keyring::MambembeKeyringError;
use mambembe_lib::{
    models::{
        AuthenticatorToken, CheckRegistrationStatus, CheckStatusResponse, RegisterDeviceResponse,
    },
    AuthyClient,
};

#[derive(Debug, StructOpt)]
enum Config {
    RegisterDevice {
        #[structopt(short, long)]
        device_name: String,
        #[structopt(short, long)]
        phone: String,
    },
    ListServices {},
    GetToken {
        #[structopt(short, long, help = "fuzzy search a service by its name")]
        service_name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;
    work().await?;
    Ok(())
}

#[instrument]
async fn work() -> Result<()> {
    let config = Config::from_args();
    match config {
        Config::RegisterDevice { phone, device_name } => {
            if get_client_from_file().await.is_ok() {
                eprintln!("You already have a registered device.");
                exit(1);
            }
            println!("Type your password: ");
            let backup_password = BufReader::new(io::stdin())
                .lines()
                .next_line()
                .await
                .expect("failed to read backup password")
                .expect("no password provided");

            let client = get_new_client(&phone, &device_name, &backup_password).await?;
            save_client_configuration(&client)?;
        }
        Config::ListServices {} => {
            let client = get_saved_client()?;
            let services = client.list_authenticator_tokens().await?;
            // As this is fresh, lets update our keyring
            mambembe_keyring::set(&services).map_err(wrap_keyring_error)?;

            for service in services {
                println!(
                    "Name: {:?} Account type: {:?}",
                    service.name, service.account_type
                );
            }
        }
        Config::GetToken { service_name } => {
            let client = get_saved_client()?;
            let mut services: Vec<AuthenticatorToken> = match mambembe_keyring::get() {
                Ok(services) => services,
                Err(MambembeKeyringError::NoPasswordFound) => {
                    let services = client.list_authenticator_tokens().await?;
                    mambembe_keyring::set(&services).unwrap();
                    services
                }
                Err(err) => return Err(wrap_keyring_error(err)),
            };
            let matcher = SkimMatcherV2::default();

            let filtered: Vec<_> = services
                .iter_mut()
                .filter(|t| matcher.fuzzy_match(&t.name, &service_name).is_some())
                .collect();

            for service in filtered {
                client.initialize_authenticator_token(&mut *service)?;
                let token = client.get_otp_token(service).await?;
                println!(
                    "Service: {:?} Token: {:?} Type: {:#?}",
                    service.name, token, 1
                )
            }
        }
    }

    // client.check_current_device().await?;
    // client.sync_time_with_server().await?;
    // // client.check_current_device_keys().await?;
    // client.fetch_private_keys().await?;
    // save_client_configuration(&client).await?;
    // let services = client.list_authenticator_tokens().await?;
    // for token in services {
    //     println!("{:#?}", token);
    //     println!("{:?} -> {:?}", token.name, client.get_otp_token(&token)?);
    // }
    Ok(())
}

#[instrument]
async fn get_client_from_file() -> Result<AuthyClient> {
    AuthyClient::from_file()
        .await
        .wrap_err("failed to get client from a file")
}

#[instrument]
async fn get_new_client(
    phone: &str,
    device_name: &str,
    backup_password: &str,
) -> Result<AuthyClient> {
    let mut client = AuthyClient::new(device_name, backup_password)?;
    match client.check_user_status(phone).await? {
        CheckStatusResponse::RegisterDevice => {}
        CheckStatusResponse::RegisterAccount => {
            eprintln!("Account registration is not implemented yet");
            exit(1);
        }
    }

    let RegisterDeviceResponse::RegistrationPending(request_id) = client.register_device().await?;

    let pin = loop {
        let response = client.check_registration(&request_id).await?;

        match response {
            CheckRegistrationStatus::Accepted(pin) => break pin,
            CheckRegistrationStatus::Pending => {}
        };
        info!("Waiting for device registration");
        sleep(Duration::from_secs(10)).await;
    };

    client.complete_registration(&pin).await?;

    save_client_configuration(&client)?;
    Ok(client)
}

fn save_client_configuration(client: &AuthyClient) -> Result<()> {
    mambembe_keyring::set(client)
        .map_err(wrap_keyring_error)
        .wrap_err("failed to save client configuration")
}

fn get_saved_client() -> Result<AuthyClient> {
    mambembe_keyring::get::<AuthyClient>()
        .map_err(wrap_keyring_error)
        .wrap_err("failed to fetch saved client")
}

fn wrap_keyring_error(e: MambembeKeyringError) -> Report {
    // This is done because windows::Error does not allow Send
    eyre!(e)
}
