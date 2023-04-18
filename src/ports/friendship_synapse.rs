use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::entities::friendship_history::FriendshipHistory;

#[derive(Deserialize, Serialize, PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub enum FriendshipEvent {
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

lazy_static::lazy_static! {
    static ref VALID_FRIENDSHIP_EVENT_TRANSITIONS: HashMap<FriendshipEvent, Vec<Option<FriendshipEvent>>> = {
        let mut m = HashMap::new();

        // This means that request is valid new event for all the specified events
        // (meaning that that's the previous event)
        m.insert(FriendshipEvent::REQUEST, vec![None, Some(FriendshipEvent::CANCEL), Some(FriendshipEvent::REJECT), Some(FriendshipEvent::DELETE)]);
        m.insert(FriendshipEvent::CANCEL, vec![Some(FriendshipEvent::REQUEST)]);
        m.insert(FriendshipEvent::ACCEPT, vec![Some(FriendshipEvent::REQUEST)]);
        m.insert(FriendshipEvent::REJECT, vec![Some(FriendshipEvent::REQUEST)]);
        m.insert(FriendshipEvent::DELETE, vec![Some(FriendshipEvent::ACCEPT)]);

        m
    };
}

impl FriendshipEvent {
    /// Validate the new event is valid and different from the last recorded.
    pub fn validate_new_event_is_valid(
        current_event: &Option<FriendshipEvent>,
        new_event: FriendshipEvent,
    ) -> bool {
        if current_event.map_or(true, |event| event.is_different(&new_event)) {
            let valid_transitions = VALID_FRIENDSHIP_EVENT_TRANSITIONS.get(&new_event).unwrap();
            valid_transitions.contains(current_event)
        } else {
            false
        }
    }

    pub fn is_different(&self, new_event: &Self) -> bool {
        self != new_event
    }
}

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
