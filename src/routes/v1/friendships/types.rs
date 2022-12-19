use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FriendshipsResponse {
    pub friendships: Vec<FriendshipFriend>,
}

impl FriendshipsResponse {
    pub fn new(addresses: Vec<&str>) -> Self {
        let friends = addresses.iter().map(|address| FriendshipFriend {
            address: address.to_string(),
        });

        Self {
            friendships: friends.collect(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FriendshipFriend {
    address: String,
}