use urlencoding::encode;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{collections::HashMap, time::SystemTime};

use crate::{
    api::routes::synapse::room_events::{
        JoinedRoomsResponse, RoomEventRequestBody, RoomEventResponse, RoomJoinResponse,
    },
    domain::{error::CommonError, friendship_event::FriendshipEvent},
};

#[derive(Deserialize, Serialize)]
pub struct AccountDataContentResponse {
    #[serde(flatten)]
    #[serde(rename = "m.direct")]
    pub direct: HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RoomIdResponse {
    pub room_id: String,
    servers: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SynapseComponent {
    pub synapse_url: String,
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

#[derive(Deserialize, Serialize, Debug)]
pub struct RoomMember {
    pub state_key: String,
    pub social_user_id: Option<String>, // social_user_id is not present in synapse
    pub user_id: String,
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

#[derive(Deserialize, Serialize)]
pub struct CreateRoomOpts {
    pub room_alias_name: String,
    pub preset: String,
    pub invite: Vec<String>,
    pub is_direct: bool,
}

#[derive(Deserialize, Serialize)]
pub struct InviteUserRequest {
    pub user_id: String,
}

#[derive(Deserialize, Serialize)]
pub struct CreateRoomResponse {
    pub room_id: String,
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

    /// https://spec.matrix.org/v1.3/client-server-api/#get_matrixclientv3joined_rooms
    #[tracing::instrument(name = "get joined rooms > Synapse components", skip(token))]
    pub async fn get_joined_rooms(&self, token: &str) -> Result<JoinedRoomsResponse, CommonError> {
        let path = "/_matrix/client/r0/joined_rooms".to_string();

        Self::authenticated_get_request(&path, token, &self.synapse_url).await
    }

    /// https://spec.matrix.org/v1.3/client-server-api/#joining-rooms
    #[tracing::instrument(name = "join room > Synapse components", skip(token))]
    pub async fn join_room(
        &self,
        token: &str,
        room_id: &str,
    ) -> Result<RoomJoinResponse, CommonError> {
        let encoded_room_id = encode(room_id).to_string();
        let path = format!("/_matrix/client/r0/rooms/{encoded_room_id}/join");

        Self::authenticated_post_request(&path, token, &self.synapse_url, ()).await
    }

    #[tracing::instrument(name = "put room event > Synapse components", skip(token))]
    pub async fn store_room_event(
        &self,
        token: &str,
        room_id: &str,
        room_event: FriendshipEvent,
        room_message_body: Option<&str>,
    ) -> Result<RoomEventResponse, CommonError> {
        let encoded_room_id = encode(room_id).to_string();
        let path =
            format!("/_matrix/client/r0/rooms/{encoded_room_id}/state/org.decentraland.friendship");

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

    #[tracing::instrument(
        name = "put send message event to the given room > Synapse components",
        skip(token)
    )]
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

        let encoded_room_id = encode(room_id).to_string();
        let path =
            format!("/_matrix/client/r0/rooms/{encoded_room_id}/send/m.room.message/{txn_id}");

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

    #[tracing::instrument(name = "get_room_members > Synapse components", skip(token))]
    pub async fn get_room_members(
        &self,
        token: &str,
        room_id: &str,
    ) -> Result<RoomMembersResponse, CommonError> {
        let encoded_room_id = encode(room_id).to_string();
        let path = format!("/_matrix/client/r0/rooms/{encoded_room_id}/members");
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
                .for_each(|room_member| {
                    room_member.social_user_id =
                        Some(clean_synapse_user_id(&room_member.state_key));
                });

            res
        })
    }

    /// https://spec.matrix.org/v1.3/client-server-api/#creation
    #[tracing::instrument(name = "create_private_room > Synapse components", skip(token))]
    pub async fn create_private_room(
        &self,
        token: &str,
        synapse_user_ids: Vec<&str>,
        room_alias_name: &str,
    ) -> Result<CreateRoomResponse, CommonError> {
        let path = "/_matrix/client/r0/createRoom".to_string();

        let invite = synapse_user_ids
            .iter()
            .map(|id| id.to_string().to_lowercase())
            .collect();

        Self::authenticated_post_request(
            &path,
            token,
            &self.synapse_url,
            &CreateRoomOpts {
                room_alias_name: room_alias_name.to_string(),
                preset: "trusted_private_chat".to_string(),
                invite,
                is_direct: true,
            },
        )
        .await
    }

    /// Sets the account data content for the given user id
    /// Check out the details [here](https://spec.matrix.org/v1.3/client-server-api/#get_matrixclientv3useruseridaccount_datatype)
    pub async fn set_account_data(
        &self,
        token: &str,
        synapse_user_id: &str,
        direct_room_map: HashMap<String, Vec<String>>,
    ) -> Result<(), CommonError> {
        let encoded_synapse_user_id = encode(synapse_user_id).to_string();
        let path: String =
            format!("/_matrix/client/r0/user/{encoded_synapse_user_id}/account_data/m.direct");

        let result: Result<HashMap<String, Vec<String>>, _> =
            Self::authenticated_put_request::<HashMap<String, Vec<String>>, _>(
                &path,
                token,
                &self.synapse_url,
                direct_room_map,
            )
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Retrieves the account data content for the given user id
    /// Check out the details [here](https://spec.matrix.org/v1.3/client-server-api/#put_matrixclientv3useruseridaccount_datatype)
    pub async fn get_account_data(
        &self,
        token: &str,
        synapse_user_id: &str,
    ) -> Result<AccountDataContentResponse, CommonError> {
        let encoded_synapse_user_id = encode(synapse_user_id).to_string();
        let path: String =
            format!("/_matrix/client/r0/user/{encoded_synapse_user_id}/account_data/m.direct");

        Self::authenticated_get_request(&path, token, &self.synapse_url).await
    }

    pub async fn get_room_id_for_alias(
        &self,
        token: &str,
        alias: &str,
        synapse: &SynapseComponent,
    ) -> Result<RoomIdResponse, CommonError> {
        let encoded_alias = full_encoded_alias(alias, synapse);
        let path = format!("/_matrix/client/r0/directory/room/{encoded_alias}");

        Self::authenticated_get_request(&path, token, &self.synapse_url).await
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

    async fn authenticated_post_request<T: DeserializeOwned, S: Serialize>(
        path: &str,
        token: &str,
        synapse_url: &str,
        body: S,
    ) -> Result<T, CommonError> {
        let url = format!("{synapse_url}{path}");
        let client = reqwest::Client::new();
        let response = client
            .post(url)
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
                    log::warn!("[Synapse] error reading synapse response {}", err);
                    return Err(CommonError::Unknown("".to_owned()));
                }

                let text = text.unwrap();
                let response = serde_json::from_str::<T>(&text);

                response.map_err(|_err| Self::parse_and_return_error(&text))
            }
            Err(err) => {
                log::warn!("[Synapse] error connecting to synapse {}", err);
                Err(CommonError::Unknown("".to_owned()))
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
                "M_UNKNOWN_TOKEN" => CommonError::Unauthorized("".to_owned()),
                "M_MISSING_TOKEN" => CommonError::Unauthorized("".to_owned()),
                "M_LIMIT_EXCEEDED" => CommonError::TooManyRequests("".to_owned()),
                _ => CommonError::Unknown("".to_owned()),
            },
            Err(err) => {
                log::warn!("error parsing synapse error {}", err);
                CommonError::Unknown("".to_owned())
            }
        }
    }
}

/// This function is used when getting the room by alias (full alias: like '#wombat:example.com')
/// and as it's part of the query parameter it must be encoded
///
/// Returns the encoded room alias name as a string, created from the sorted and joined user addresses in `joined_addresses`.
///
/// We need to build the room alias in this way because we're leveraging the room creation process from Matrix + SDK.
/// It follows the pattern:
/// `#{sorted and joined addresses}:decentraland.{domain}`
/// where `sorted and joined addresses` are the addresses of the two users concatenated and sorted,
/// and `domain` is the domain of the Synapse server.
fn full_encoded_alias(joined_addresses: &str, synapse: &SynapseComponent) -> String {
    let full_alias = format!(
        "#{}:decentraland.{}",
        joined_addresses,
        extract_domain(&synapse.synapse_url)
    );
    encode(&full_alias).to_string()
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

/// Extracts the domain from a URL.
///
/// Returns a string representing the domain extracted from the URL. If the URL cannot
/// be split into parts or the domain is empty, the function returns the default value
/// `zone`.
pub fn extract_domain(url: &str) -> &str {
    let splited_domain: Vec<&str> = url.split('.').collect();
    let last_part = splited_domain.last().unwrap_or(&"zone");

    if last_part == &"zone" || last_part == &"org" {
        last_part
    } else {
        "zone"
    }
}

/// Gets the synapse user id for the given user id
///
/// @example
/// from: '0x1111ada11111'
/// to: '@0x1111ada11111:decentraland.org'
pub fn user_id_as_synapse_user_id(user_id: &str, synapse_url: &str) -> String {
    let result = format!("@{}:decentraland.{}", user_id, extract_domain(synapse_url));
    result
}

#[cfg(test)]
mod tests {
    use super::{clean_synapse_user_id, user_id_as_synapse_user_id};

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

    #[test]
    fn user_id_as_synapse_id_for_plain_user() {
        let res = user_id_as_synapse_user_id("0x1111ada11111", "");

        assert_eq!(res, "@0x1111ada11111:decentraland.zone");
    }
}
