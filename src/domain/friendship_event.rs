use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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

impl FriendshipEvent {
    pub fn as_str(&self) -> &'static str {
        match *self {
            FriendshipEvent::REQUEST => "request",
            FriendshipEvent::CANCEL => "cancel",
            FriendshipEvent::ACCEPT => "accept",
            FriendshipEvent::REJECT => "reject",
            FriendshipEvent::DELETE => "delete",
        }
    }
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

    fn is_different(&self, new_event: &Self) -> bool {
        self != new_event
    }
}

#[cfg(test)]
mod tests {
    use super::FriendshipEvent;

    #[test]
    fn none_and_then() {
        let none_request =
            FriendshipEvent::validate_new_event_is_valid(&None, FriendshipEvent::REQUEST);
        assert!(none_request);

        let none_reject =
            FriendshipEvent::validate_new_event_is_valid(&None, FriendshipEvent::REJECT);
        assert!(!none_reject);

        let none_accept =
            FriendshipEvent::validate_new_event_is_valid(&None, FriendshipEvent::ACCEPT);
        assert!(!none_accept);

        let none_cancel =
            FriendshipEvent::validate_new_event_is_valid(&None, FriendshipEvent::CANCEL);
        assert!(!none_cancel);

        let none_delete =
            FriendshipEvent::validate_new_event_is_valid(&None, FriendshipEvent::DELETE);
        assert!(!none_delete);
    }

    #[test]
    fn request_and_then() {
        let request_accept = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::REQUEST),
            FriendshipEvent::ACCEPT,
        );
        assert!(request_accept);

        let request_reject = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::REQUEST),
            FriendshipEvent::REJECT,
        );
        assert!(request_reject);

        let request_cancel = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::REQUEST),
            FriendshipEvent::CANCEL,
        );
        assert!(request_cancel);

        let request_request = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::REQUEST),
            FriendshipEvent::REQUEST,
        );
        assert!(!request_request);

        let request_delete = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::REQUEST),
            FriendshipEvent::DELETE,
        );
        assert!(!request_delete);
    }

    #[test]
    fn accept_and_then() {
        let accept_accept = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::ACCEPT),
            FriendshipEvent::ACCEPT,
        );
        assert!(!accept_accept);

        let accept_reject = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::ACCEPT),
            FriendshipEvent::REJECT,
        );
        assert!(!accept_reject);

        let accept_cancel = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::ACCEPT),
            FriendshipEvent::CANCEL,
        );
        assert!(!accept_cancel);

        let accept_request = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::ACCEPT),
            FriendshipEvent::REQUEST,
        );
        assert!(!accept_request);

        let accept_delete = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::ACCEPT),
            FriendshipEvent::DELETE,
        );
        assert!(accept_delete);
    }

    #[test]
    fn reject_and_then() {
        let reject_accept = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::REJECT),
            FriendshipEvent::ACCEPT,
        );
        assert!(!reject_accept);

        let reject_reject = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::REJECT),
            FriendshipEvent::REJECT,
        );
        assert!(!reject_reject);

        let reject_cancel = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::REJECT),
            FriendshipEvent::CANCEL,
        );
        assert!(!reject_cancel);

        let reject_request = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::REJECT),
            FriendshipEvent::REQUEST,
        );
        assert!(reject_request);

        let reject_delete = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::REJECT),
            FriendshipEvent::DELETE,
        );
        assert!(!reject_delete);
    }

    #[test]
    fn cancel_and_then() {
        let cancel_accept = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::CANCEL),
            FriendshipEvent::ACCEPT,
        );
        assert!(!cancel_accept);

        let cancel_reject = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::CANCEL),
            FriendshipEvent::REJECT,
        );
        assert!(!cancel_reject);

        let cancel_cancel = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::CANCEL),
            FriendshipEvent::CANCEL,
        );
        assert!(!cancel_cancel);

        let cancel_request = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::CANCEL),
            FriendshipEvent::REQUEST,
        );
        assert!(cancel_request);

        let cancel_delete = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::CANCEL),
            FriendshipEvent::DELETE,
        );
        assert!(!cancel_delete);
    }

    #[test]
    fn delete_and_then() {
        let delete_accept = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::DELETE),
            FriendshipEvent::ACCEPT,
        );
        assert!(!delete_accept);

        let delete_reject = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::DELETE),
            FriendshipEvent::REJECT,
        );
        assert!(!delete_reject);

        let delete_cancel = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::DELETE),
            FriendshipEvent::CANCEL,
        );
        assert!(!delete_cancel);

        let delete_request = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::DELETE),
            FriendshipEvent::REQUEST,
        );
        assert!(delete_request);

        let delete_delete = FriendshipEvent::validate_new_event_is_valid(
            &Some(FriendshipEvent::DELETE),
            FriendshipEvent::DELETE,
        );
        assert!(!delete_delete);
    }
}
