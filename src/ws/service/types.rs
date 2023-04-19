use crate::{
    components::database::DatabaseComponent,
    entities::{
        friendship_event::FriendshipEvent, friendship_history::FriendshipHistoryRepository,
        friendships::FriendshipsRepository,
    },
};

pub struct EventResponse {
    pub event_id: String,
}

pub struct EventPayload {
    pub friendship_event: FriendshipEvent,
    pub second_user: String,
    pub request_event_message_body: Option<String>,
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
