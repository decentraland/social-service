use serde::{Deserialize, Serialize};

use crate::{
    components::database::DatabaseComponent,
    entities::{
        friendship_history::FriendshipHistoryRepository, friendships::FriendshipsRepository,
    },
};

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub enum FriendshipEventWs {
    #[serde(rename = "request")]
    REQUEST, // Send a friendship request
    #[serde(rename = "cancel")]
    CANCEL, // Cancel a friendship request
    #[serde(rename = "accept")]
    ACCEPT, // Accept a friendship request
    #[serde(rename = "reject")]
    REJECT, // Reject a friendship request
    #[serde(rename = "delete")]
    DELETE, // Delete an existing friendship
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum FriendshipStatusWs {
    Friends,
    Requested(String),
    NotFriends,
}

pub struct FriendshipPortsWs<'a> {
    pub db: &'a DatabaseComponent,
    pub friendships_repository: &'a FriendshipsRepository,
    pub friendship_history_repository: &'a FriendshipHistoryRepository,
}

pub struct RoomInfoWs<'a> {
    pub room_event: FriendshipEventWs,
    pub room_message_body: Option<&'a str>,
    pub room_id: &'a str,
}
