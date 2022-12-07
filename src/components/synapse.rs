use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[cfg_attr(test, faux::create)]
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

#[cfg_attr(test, faux::methods)]
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

    pub async fn who_am_i(&self, token: &str) -> Result<WhoAmIResponse, String> {
        let who_am_i_url = format!("{}{}", self.synapse_url, WHO_AM_I_URI);
        let client = reqwest::Client::new();
        let result: Result<WhoAmIResponse, String> = match client
            .get(who_am_i_url)
            .header("authorization", token)
            .send()
            .await
        {
            Ok(response) => match response.json::<WhoAmIResponse>().await {
                Ok(json) => Ok(json),
                Err(err) => Err(err.to_string()),
            },
            Err(err) => Err(err.to_string()),
        };

        result
    }
}
