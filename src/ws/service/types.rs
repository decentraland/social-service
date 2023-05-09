use crate::{
    components::database::DatabaseComponent,
    domain::friendship_event::FriendshipEvent,
    entities::{
        friendship_history::FriendshipHistoryRepository, friendships::FriendshipsRepository,
    },
};

pub struct EventResponse {
    pub user_id: String,
}

pub struct EventPayload {
    pub friendship_event: FriendshipEvent,
    pub second_user: String,
    pub request_event_message_body: Option<String>,
}
