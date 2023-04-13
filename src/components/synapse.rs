use std::{collections::HashMap, time::SystemTime};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::api::routes::{
    synapse::room_events::{FriendshipEvent, RoomEventRequestBody, RoomEventResponse},
    v1::error::CommonError,
};

#[derive(Debug, Clone)]
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
    pub social_user_id: Option<String>, // social_user_id is not present in synapse
}

#[derive(Deserialize, Serialize)]
pub struct SynapseErrorResponse {
    pub errcode: String,
    pub error: Option<String>,
    pub soft_logout: Option<bool>,
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
    pub social_user_id: Option<String>, // social_user_id is not present in synapse
    pub access_token: String,
    pub device_id: String,
    pub home_server: String,
    pub well_known: HashMap<String, HashMap<String, String>>,
}

#[derive(Deserialize, Serialize)]
pub struct RoomMember {
    pub state_key: String,
    pub social_user_id: Option<String>, // social_user_id is not present in synapse
    pub room_id: String,
    pub r#type: String,
}

#[derive(Deserialize, Serialize)]
pub struct RoomMembersResponse {
    pub chunk: Vec<RoomMember>,
}

#[derive(Deserialize, Serialize)]
pub struct MessageRequestEventBody {
    pub msgtype: String,
    pub body: String,
}

impl SynapseComponent {
    pub fn new(url: String) -> Self {
        if url.is_empty() {
            panic!("missing synapse URL")
        }

        Self { synapse_url: url }
    }

    pub async fn get_version(&self) -> Result<VersionResponse, CommonError> {
        Self::get_request::<VersionResponse>(VERSION_URI, &self.synapse_url).await
    }

    pub async fn who_am_i(&self, token: &str) -> Result<WhoAmIResponse, CommonError> {
        let result = Self::authenticated_get_request::<WhoAmIResponse>(
            WHO_AM_I_URI,
            token,
            self.synapse_url.as_str(),
        )
        .await;

        result.map(|mut res| {
            res.social_user_id = Some(clean_synapse_user_id(&res.user_id));
            res
        })
    }

    #[tracing::instrument(name = "login function > Synapse components")]
    pub async fn login(
        &self,
        request: SynapseLoginRequest,
    ) -> Result<SynapseLoginResponse, CommonError> {
        let login_url = format!("{}{}", self.synapse_url, LOGIN_URI);
        let client = reqwest::Client::new();
        let result = client
            .post(login_url)
            .json::<SynapseLoginRequest>(&request)
            .send()
            .await;

        let response = Self::process_synapse_response::<SynapseLoginResponse>(result).await;

        response.map(|mut res| {
            res.social_user_id = Some(clean_synapse_user_id(&res.user_id));
            res
        })
    }

    #[tracing::instrument(name = "put room event > Synapse components")]
    pub async fn store_room_event(
        &self,
        token: &str,
        room_id: &str,
        room_event: FriendshipEvent,
        room_message_body: Option<&str>,
    ) -> Result<RoomEventResponse, CommonError> {
        let path = format!("/_matrix/client/r0/rooms/{room_id}/state/org.decentraland.friendship");

        Self::authenticated_put_request(
            &path,
            token,
            &self.synapse_url,
            &RoomEventRequestBody {
                r#type: room_event,
                message: room_message_body.map(|s| s.to_string()),
            },
        )
        .await
    }

    #[tracing::instrument(name = "put send message event to the given room > Synapse components")]
    pub async fn send_message_event_given_room(
        &self,
        token: &str,
        room_id: &str,
        room_event: FriendshipEvent,
        room_message_body: &str,
    ) -> Result<RoomEventResponse, CommonError> {
        // The transaction ID for this event.
        // Clients should generate an ID unique across requests with the same access token;
        // it will be used by the server to ensure idempotency of requests.
        let txn_id = format!(
            "m.{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        let path = format!("/_matrix/client/r0/rooms/{room_id}/send/m.room.message/{txn_id}");

        Self::authenticated_put_request(
            &path,
            token,
            &self.synapse_url,
            &MessageRequestEventBody {
                msgtype: "m.text".to_string(),
                body: room_message_body.to_string(),
            },
        )
        .await
    }

    #[tracing::instrument(name = "get_room_members > Synapse components")]
    pub async fn get_room_members(
        &self,
        token: &str,
        room_id: &str,
    ) -> Result<RoomMembersResponse, CommonError> {
        let path = format!("/_matrix/client/r0/rooms/{room_id}/members");
        let response = Self::authenticated_get_request::<RoomMembersResponse>(
            &path,
            token,
            self.synapse_url.as_str(),
        )
        .await;

        response.map(|mut res| {
            res.chunk
                .iter_mut()
                .filter(|room_member| room_member.state_key.starts_with('@'))
                .for_each(|mut room_member| {
                    room_member.social_user_id =
                        Some(clean_synapse_user_id(&room_member.state_key));
                });

            res
        })
    }

    async fn get_request<T: DeserializeOwned>(
        path: &str,
        synapse_url: &str,
    ) -> Result<T, CommonError> {
        let url = format!("{synapse_url}{path}");
        let client = reqwest::Client::new();
        let response = client.get(url).send().await;

        Self::process_synapse_response(response).await
    }

    async fn authenticated_put_request<T: DeserializeOwned, S: Serialize>(
        path: &str,
        token: &str,
        synapse_url: &str,
        body: S,
    ) -> Result<T, CommonError> {
        let url = format!("{synapse_url}{path}");
        let client = reqwest::Client::new();
        let response = client
            .put(url)
            .json(&body)
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await;

        Self::process_synapse_response::<T>(response).await
    }

    async fn authenticated_get_request<T: DeserializeOwned>(
        path: &str,
        token: &str,
        synapse_url: &str,
    ) -> Result<T, CommonError> {
        let url = format!("{synapse_url}{path}");
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await;

        Self::process_synapse_response::<T>(response).await
    }

    async fn process_synapse_response<T: DeserializeOwned>(
        response: Result<reqwest::Response, reqwest::Error>,
    ) -> Result<T, CommonError> {
        match response {
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
                "M_FORBIDDEN" => {
                    CommonError::Forbidden(error.error.unwrap_or("Forbidden".to_string()))
                }
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

/// Get the local part of the userId from matrixUserId
///
/// @example
/// from: '@0x1111ada11111:decentraland.org'
/// to: '0x1111ada11111'
pub fn clean_synapse_user_id(user_id: &str) -> String {
    let at_position = user_id.chars().position(|char| char == '@');
    // this means that the id comes from matrix
    if let Some(at_position) = at_position {
        if at_position == 0 {
            // todo!: validate that the content is indeed an address, otherwise leave it as is
            let split_server = user_id.split(':').collect::<Vec<&str>>();

            return split_server[0].replace('@', "");
        }
    }

    user_id.to_string()
}

#[cfg(test)]
mod tests {
    use super::clean_synapse_user_id;

    #[test]
    fn clear_should_obtain_expected_string_for_synapse_user() {
        let res = clean_synapse_user_id("@0x1111ada11111:decentraland.org");

        assert_eq!(res, "0x1111ada11111");
    }

    #[test]
    fn clear_should_obtain_expected_string_for_plain_user() {
        let res = clean_synapse_user_id("0x1111ada11111");

        assert_eq!(res, "0x1111ada11111");
    }
}
