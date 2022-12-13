use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[async_trait::async_trait]
pub trait SynapseComponent {
    fn new(url: String) -> Self
    where
        Self: Sized;
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

#[async_trait::async_trait]
impl SynapseComponent for Synapse {
    fn new(url: String) -> Self {
        Self { synapse_url: url }
    }
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
