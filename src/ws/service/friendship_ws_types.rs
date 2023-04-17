use crate::{
    api::routes::synapse::room_events::FriendshipEvent,
    components::database::DatabaseComponent,
    entities::{
        friendship_history::FriendshipHistoryRepository, friendships::FriendshipsRepository,
    },
};

pub struct EventResponse {
    pub event_id: String,
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
    pub room_event: FriendshipEvent,
    pub room_message_body: Option<&'a str>,
    pub room_id: &'a str,
}
