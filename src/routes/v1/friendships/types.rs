use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct FriendshipsResponse {
    friendships: Vec<Friend>,
}

impl FriendshipsResponse {
    pub fn new(addresses: Vec<&str>) -> Self {
        let friends = addresses.iter().map(|address| Friend {
            address: address.to_string(),
        });

        Self {
            friendships: friends.collect(),
        }
    }
}

#[derive(Debug, Default, Serialize)]
struct Friend {
    address: String,
}
