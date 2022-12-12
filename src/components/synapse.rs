use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[async_trait::async_trait]
pub trait SynapaseComponent {
    async fn get_version(&self) -> Result<VersionResponse, String>;
}

#[derive(Debug)]
pub struct Synapse {
    synapse_url: String,
}

pub const VERSION_URI: &str = "/_matrix/client/versions";

#[derive(Deserialize, Serialize)]
pub struct VersionResponse {
    versions: Vec<String>,
    unstable_features: HashMap<String, bool>,
}

impl Synapse {
    pub fn new(url: String) -> Self {
        if url.is_empty() {
            panic!("missing synapse URL")
        }

        Self { synapse_url: url }
    }
}

#[async_trait::async_trait]
impl SynapaseComponent for Synapse {
    async fn get_version(&self) -> Result<VersionResponse, String> {
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
}
