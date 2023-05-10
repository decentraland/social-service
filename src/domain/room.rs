use super::friendship_event::FriendshipEvent;

pub struct RoomInfo<'a> {
    pub room_event: FriendshipEvent,
    pub room_message_body: Option<&'a str>,
    pub room_id: &'a str,
}
