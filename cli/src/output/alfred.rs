use std::str::FromStr;

use serde::Serialize;

use crate::{output::ServiceToken, Output};

#[derive(Debug, Serialize)]
pub(crate) struct Alfred {
    items: Vec<AlfredItem>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AlfredItem {
    title: String,
    #[serde(rename = "arg")]
    token: String,
}

impl Alfred {
    pub fn from_service_token(data: Vec<ServiceToken>) -> Self {
        Self {
            items: data
                .into_iter()
                .map(|item| AlfredItem {
                    title: item.service,
                    token: item.token,
                })
                .collect(),
        }
    }
}

impl FromStr for Output {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "text" => Self::PlainText,
            "json" => Self::Json,
            "alfred" => Self::Alfred,
            other => return Err(format!("output {} is invalid", other)),
        })
    }
}
impl ToString for Output {
    fn to_string(&self) -> String {
        match self {
            Output::PlainText => "text",
            Output::Json => "json",
            Output::Alfred => "alfred",
        }
        .to_string()
    }
}
