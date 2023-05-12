use crate::entities::friendship_history::FriendshipHistory;

use super::friendship_event::FriendshipEvent;

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum FriendshipStatus {
    Friends,
    Requested(String),
    NotFriends,
}

impl FriendshipStatus {
    pub fn from_history_event(history: Option<FriendshipHistory>) -> Self {
        if history.is_none() {
            return FriendshipStatus::NotFriends;
        }

        let history = history.unwrap();

        match history.event {
            FriendshipEvent::REQUEST => FriendshipStatus::Requested(history.acting_user),
            FriendshipEvent::CANCEL => FriendshipStatus::NotFriends,
            FriendshipEvent::ACCEPT => FriendshipStatus::Friends,
            FriendshipEvent::REJECT => FriendshipStatus::NotFriends,
            FriendshipEvent::DELETE => FriendshipStatus::NotFriends,
        }
    }
}
