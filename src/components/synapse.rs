use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::routes::v1::error::CommonError;

#[cfg_attr(any(test, feature = "faux"), faux::create)]
#[derive(Debug)]
pub struct SynapseComponent {
    synapse_url: String,
}

pub const VERSION_URI: &str = "/_matrix/client/versions";
pub const WHO_AM_I_URI: &str = "/_matrix/client/v3/account/whoami";

#[derive(Deserialize, Serialize)]
pub struct VersionResponse {
    pub versions: Vec<String>,
    pub unstable_features: HashMap<String, bool>,
}

#[derive(Deserialize, Serialize)]
pub struct WhoAmIResponse {
    pub user_id: String,
}

#[derive(Deserialize, Serialize)]
pub struct SynapseErrorResponse {
    pub errcode: String,
    pub error: String,
    pub soft_logout: bool,
}

#[cfg_attr(any(test, feature = "faux"), faux::methods)]
impl SynapseComponent {
    pub fn new(url: String) -> Self {
        if url.is_empty() {
            panic!("missing synapse URL")
        }

        Self { synapse_url: url }
    }

    pub async fn get_version(&self) -> Result<VersionResponse, String> {
        let version_url = format!("{}{}", self.synapse_url, VERSION_URI);
        let result: Result<VersionResponse, String> = match reqwest::get(version_url).await {
            Ok(response) => match response.json::<VersionResponse>().await {
                Ok(json) => Ok(json),
                Err(err) => Err(err.to_string()),
            },
            Err(err) => Err(err.to_string()),
        };

        result
    }

    pub async fn who_am_i(&self, token: &str) -> Result<WhoAmIResponse, CommonError> {
        let who_am_i_url = format!("{}{}", self.synapse_url, WHO_AM_I_URI);
        let client = reqwest::Client::new();
        match client
            .get(who_am_i_url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(response) => {
                let text = response.text().await;
                if let Err(err) = text {
                    log::warn!("error reading synapse response {}", err);
                    return Err(CommonError::Unknown);
                }

                let text = text.unwrap();

                let who_am_i_response = serde_json::from_str::<WhoAmIResponse>(&text);

                match who_am_i_response {
                    Ok(who_am_i) => Ok(who_am_i),
                    Err(_) => Err(SynapseComponent::parse_and_return_error(&text)),
                }
            }
            Err(err) => {
                log::warn!("error connecting to synapse {}", err);
                Err(CommonError::Unknown)
            }
        }
    }

    fn parse_and_return_error(text: &str) -> CommonError {
        let error_response = serde_json::from_str::<SynapseErrorResponse>(&text);

        match error_response {
            Ok(error) => match error.errcode.as_str() {
                "M_UNKNOWN_TOKEN" => CommonError::Unauthorized,
                "M_MISSING_TOKEN" => CommonError::Unauthorized,
                "M_LIMIT_EXCEEDED" => CommonError::TooManyRequests,
                _ => CommonError::Unknown,
            },
            Err(err) => {
                log::warn!("error parsing synapse error {}", err);
                CommonError::Unknown
            }
        }
    }
}
