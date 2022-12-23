use std::collections::HashMap;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::routes::v1::error::CommonError;

#[cfg_attr(any(test, feature = "faux"), faux::create)]
#[derive(Debug)]
pub struct SynapseComponent {
    synapse_url: String,
}

pub const VERSION_URI: &str = "/_matrix/client/versions";
pub const WHO_AM_I_URI: &str = "/_matrix/client/v3/account/whoami";
pub const LOGIN_URI: &str = "/_matrix/client/r0/login";

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
#[derive(Serialize, Deserialize, Debug)]
pub struct LoginIdentifier {
    #[serde(rename = "type")]
    pub _type: String,
    pub user: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AuthChain {
    #[serde(rename = "type")]
    pub _type: String,
    pub payload: String,
    pub signature: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SynapseLoginRequest {
    #[serde(rename = "type")]
    pub _type: String,
    pub identifier: LoginIdentifier,
    pub timestamp: String,
    pub auth_chain: Vec<AuthChain>,
}

#[derive(Serialize, Deserialize)]
pub struct SynapseLoginResponse {
    pub user_id: String,
    pub access_token: String,
    pub device_id: String,
    pub home_server: String,
    pub well_known: HashMap<String, HashMap<String, String>>,
}

#[cfg_attr(any(test, feature = "faux"), faux::methods)]
impl SynapseComponent {
    pub fn new(url: String) -> Self {
        if url.is_empty() {
            panic!("missing synapse URL")
        }

        Self { synapse_url: url }
    }

    pub async fn get_version(&self) -> Result<VersionResponse, CommonError> {
        let version_url = format!("{}{}", self.synapse_url, VERSION_URI);
        match reqwest::get(version_url).await {
            Ok(response) => {
                let text = response.text().await;
                if let Err(err) = text {
                    log::warn!("error reading synapse response {}", err);
                    return Err(CommonError::Unknown);
                }

                let text = text.unwrap();
                let get_version_response = serde_json::from_str::<VersionResponse>(&text);

                get_version_response.map_err(|_| Self::parse_and_return_error(&text))
            }
            Err(err) => {
                log::warn!("error connecting to synapse {}", err);
                Err(CommonError::Unknown)
            }
        }
    }

    #[tracing::instrument(name = "who_am_i function > Synapse components")]
    pub async fn who_am_i(&self, token: &str) -> Result<WhoAmIResponse, CommonError> {
        Self::perform_authenticated_request::<WhoAmIResponse>(
            WHO_AM_I_URI,
            token,
            self.synapse_url.as_str(),
        )
        .await
    }

    pub async fn perform_authenticated_request<T: DeserializeOwned>(
        path: &str,
        token: &str,
        synapse_url: &str,
    ) -> Result<T, CommonError> {
        let url = format!("{synapse_url}{path}");
        let client = reqwest::Client::new();
        match client
            .get(url)
            .header("Authorization", format!("Bearer {token}"))
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

                let response = serde_json::from_str::<T>(&text);

                response.map_err(|_| Self::parse_and_return_error(&text))
            }
            Err(err) => {
                log::warn!("error connecting to synapse {}", err);
                Err(CommonError::Unknown)
            }
        }
    }

    fn parse_and_return_error(text: &str) -> CommonError {
        let error_response = serde_json::from_str::<SynapseErrorResponse>(text);

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

    #[tracing::instrument(name = "login function > Synapse components")]
    pub async fn login(
        &self,
        request: SynapseLoginRequest,
    ) -> Result<SynapseLoginResponse, CommonError> {
        let login_url = format!("{}{}", self.synapse_url, LOGIN_URI);
        let client = reqwest::Client::new();
        let result = match client
            .post(login_url)
            .json::<SynapseLoginRequest>(&request)
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

                let login_response = serde_json::from_str::<SynapseLoginResponse>(&text);

                login_response.map_err(|_| SynapseComponent::parse_and_return_error(&text))
            }
            Err(err) => {
                log::warn!("error connecting to synapse {}", err);
                Err(CommonError::Unknown)
            }
        };

        result
    }
}
