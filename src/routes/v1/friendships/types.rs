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
    pub friendships: Vec<MessageRequestEvent>,
}

impl MessageRequestEventResponse {
    pub fn new(addresses: Vec<MessageRequestEvent>) -> Self {
        let friends = addresses.iter().map(|address| MessageRequestEvent {
            friendship_id: address.friendship_id.clone(),
            acting_user: address.acting_user.clone(),
            timestamp: address.timestamp,
            body: address.body.clone(),
        });

        Self {
            friendships: friends.collect(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct MessageRequestEvent {
    pub friendship_id: String,
    pub acting_user: String,
    pub timestamp: i64,
    pub body: String,
}
