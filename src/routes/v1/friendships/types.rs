use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FriendshipsResponse {
    pub friendships: Vec<FriendshipFriend>,
}

impl FriendshipsResponse {
    pub fn new(addresses: Vec<String>) -> Self {
        let friends = addresses.iter().map(|address| FriendshipFriend {
            address: address.to_string(),
        });

        Self {
            friendships: friends.collect(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct FriendshipFriend {
    pub address: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MessageRequestEventResponse {
    #[serde(rename = "messagesRequestEvents")]
    pub messages_req_events: Vec<MessageRequestEvent>,
}

impl MessageRequestEventResponse {
    pub fn new(messages_req_events: Vec<MessageRequestEvent>) -> Self {
        let messages = messages_req_events
            .iter()
            .map(|message_req_event| MessageRequestEvent {
                friendship_id: message_req_event.friendship_id.clone(),
                acting_user: message_req_event.acting_user.clone(),
                timestamp: message_req_event.timestamp,
                message: message_req_event.message.clone(),
            });

        Self {
            messages_req_events: messages.collect(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MessageRequestEvent {
    pub friendship_id: String,
    pub acting_user: String,
    pub timestamp: i64,
    pub message: String,
}
