use crate::domain::{friendship_event::FriendshipEvent, friendship_status::FriendshipStatus};

/**
* Calculates the new friendship status based on the provided friendship event and the last recorded history
* This method assumes that the transition from the last status to the new is valid and the acting user is allowed to perform it
*/
pub fn get_new_friendship_status(
    acting_user: &str,
    room_event: FriendshipEvent,
) -> FriendshipStatus {
    match room_event {
        FriendshipEvent::REQUEST => FriendshipStatus::Requested(acting_user.to_string()),
        FriendshipEvent::ACCEPT => FriendshipStatus::Friends,
        FriendshipEvent::CANCEL => FriendshipStatus::NotFriends,
        FriendshipEvent::REJECT => FriendshipStatus::NotFriends,
        FriendshipEvent::DELETE => FriendshipStatus::NotFriends,
    }
}
