mod alfred;

use color_eyre::Result;
use serde::{Deserialize, Serialize};

use crate::output::alfred::Alfred;

#[derive(Serialize)]
pub struct ServiceToken {
    pub(crate) service: String,
    pub(crate) token: String,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub(crate) enum Output {
    #[default]
    PlainText,
    Json,
    Alfred,
}

impl Output {
    pub fn print(&self, data: Vec<ServiceToken>) -> Result<()> {
        match self {
            Output::PlainText => {
                for item in data {
                    println!(
                        "Service: {:?} Token: {:?} Type: {:#?}",
                        item.service, item.token, 1
                    )
                }
            }
            Output::Json => {
                println!("{}", serde_json::to_string_pretty(&data)?)
            }
            Output::Alfred => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&Alfred::from_service_token(data))?
                )
            }
        }

        Ok(())
    }
}
