use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[cfg_attr(test, faux::create)]
#[derive(Debug)]
pub struct SynapseComponent {
    synapse_url: String,
}

pub const VERSION_URI: &str = "/_matrix/client/versions";

#[derive(Deserialize, Serialize)]
pub struct VersionResponse {
    pub versions: Vec<String>,
    pub unstable_features: HashMap<String, bool>,
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
}

#[cfg(test)]
mod tests {
    use faux::when;

    use super::*;

    #[actix_web::test]
    async fn should_get_an_empty_response() {
        let mut synapse_mock = SynapseComponent::faux();

        when!(synapse_mock.get_version).then(|_| {
            Ok(VersionResponse {
                versions: vec!["holaa".to_string()],
                unstable_features: HashMap::new(),
            })
        });

        let response = synapse_mock.get_version().await.unwrap();

        assert_eq!(response.versions[0], "holaa")
    }
}
