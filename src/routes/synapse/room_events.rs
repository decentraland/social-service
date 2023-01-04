use std::collections::HashMap;

use actix_web::{
    put,
    web::{self, Data},
    HttpMessage, HttpRequest, HttpResponse,
};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    components::{
        app::AppComponents,
        database::{DatabaseComponent, DatabaseComponentImplementation},
        synapse::{RoomMembersResponse, SynapseComponent},
    },
    entities::{
        friendship_history::{FriendshipHistory, FriendshipHistoryRepository},
        friendships::{Friendship, FriendshipRepositoryImplementation, FriendshipsRepository},
    },
    middlewares::check_auth::{Token, UserId},
    routes::v1::error::CommonError,
};

use super::errors::SynapseError;

#[derive(Deserialize, Serialize)]
pub struct RoomEventResponse {
    pub event_id: String,
}

#[derive(Deserialize, Serialize)]
pub struct RoomEventRequestBody {
    pub r#type: FriendshipEvent,
}

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
    fn validate_new_event_is_valid(
        current_event: &Option<FriendshipEvent>,
        new_event: FriendshipEvent,
    ) -> bool {
        let valid_transitions = VALID_FRIENDSHIP_EVENT_TRANSITIONS.get(&new_event).unwrap();
        valid_transitions.contains(current_event)
    }
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum FriendshipStatus {
    Friends,
    Requested(String),
    NotFriends,
}

impl FriendshipStatus {
    fn from_history_event(history: Option<FriendshipHistory>) -> Self {
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

#[put("/_matrix/client/r0/rooms/{room_id}/state/org.decentraland.friendship")]
pub async fn room_event_handler(
    req: HttpRequest,
    body: web::Json<RoomEventRequestBody>,
    room_id: web::Path<String>,
    app_data: Data<AppComponents>,
) -> Result<HttpResponse, SynapseError> {
    let (logged_in_user, token) = {
        let extensions = req.extensions();
        let logged_in_user = extensions
            .get::<UserId>()
            .expect("to have a UserId")
            .0
            .clone();

        let token = extensions
            .get::<Token>()
            .expect("To have an authentication token")
            .0
            .clone();

        (logged_in_user, token)
    };

    let response = process_room_event(
        &logged_in_user,
        &token,
        room_id.as_str(),
        body.r#type,
        &app_data.db,
        &app_data.synapse,
    )
    .await;

    if let Ok(res) = response {
        return Ok(HttpResponse::Ok().json(res));
    }

    let err = response.err().unwrap();

    Err(err)
}

async fn process_room_event(
    acting_user: &str,
    token: &str,
    room_id: &str,
    room_event: FriendshipEvent,
    db: &DatabaseComponent,
    synapse: &SynapseComponent,
) -> Result<RoomEventResponse, SynapseError> {
    // GET MEMBERS FROM SYNAPSE
    let members_result = synapse.get_room_members(token, room_id).await;
    let (address_0, address_1) = get_room_members(members_result).await?;

    let second_user = if address_0.eq_ignore_ascii_case(acting_user) {
        address_1
    } else {
        address_0
    };

    // GET LAST STATUS FROM DB
    let repos = db.db_repos.as_ref().unwrap();
    let friendship = get_friendship_from_db(&repos.friendships, acting_user, &second_user).await?;

    let last_history = get_last_history_from_db(&friendship, &repos.friendship_history).await?;

    // PROCESS NEW STATUS OF FRIENDSHIP
    let new_status = process_friendship_status(acting_user, &last_history, room_event)?;

    let current_status = FriendshipStatus::from_history_event(last_history);

    // UPDATE FRIENDSHIP ACCORDINGLY IN DB
    update_friendship_status(
        &friendship,
        acting_user,
        &second_user,
        current_status,
        new_status,
        room_event,
        db,
        &repos.friendships,
        &repos.friendship_history,
    )
    .await?;

    let res = synapse.store_room_event(token, room_id, room_event).await;

    match res {
        Ok(res) => Ok(res),
        Err(err) => Err(SynapseError::CommonError(err)),
    }
}

async fn get_room_members(
    room_members_response: Result<RoomMembersResponse, CommonError>,
) -> Result<(String, String), SynapseError> {
    match room_members_response {
        Ok(response) => {
            let members = response
                .chunk
                .iter()
                .map(|member| member.user_id.clone())
                .collect::<Vec<String>>();

            if members.len() != 2 {
                return Err(SynapseError::FriendshipNotFound);
            }

            Ok((
                members.get(0).unwrap().to_string(),
                members.get(1).unwrap().to_string(),
            ))
        }
        Err(err) => Err(SynapseError::CommonError(err)),
    }
}

async fn get_friendship_from_db(
    friendships_repository: &FriendshipsRepository,
    address_0: &str,
    address_1: &str,
) -> Result<Option<Friendship>, SynapseError> {
    let (friendship_result, _) = friendships_repository
        .get_friendship((address_0, address_1), None)
        .await;

    if friendship_result.is_err() {
        let err = friendship_result.err().unwrap();

        log::warn!("Error getting friendship in room event {}", err);
        return Err(SynapseError::CommonError(
            crate::routes::v1::error::CommonError::Unknown,
        ));
    }

    Ok(friendship_result.unwrap())
}

async fn get_last_history_from_db(
    friendship: &Option<Friendship>,
    friendship_history_repository: &FriendshipHistoryRepository,
) -> Result<Option<FriendshipHistory>, SynapseError> {
    let friendship = {
        match friendship {
            Some(friendship) => friendship,
            None => return Ok(None),
        }
    };

    let (friendship_history_result, _) = friendship_history_repository
        .get_last_history_for_friendship(friendship.id, None)
        .await;

    if friendship_history_result.is_err() {
        let err = friendship_history_result.err().unwrap();

        log::warn!("Error getting friendship history in room event {}", err);
        return Err(SynapseError::CommonError(
            crate::routes::v1::error::CommonError::Unknown,
        ));
    }

    Ok(friendship_history_result.unwrap())
}

fn process_friendship_status(
    acting_user: &str,
    last_history: &Option<FriendshipHistory>,
    room_event: FriendshipEvent,
) -> Result<FriendshipStatus, SynapseError> {
    let last_event = { last_history.as_ref().map(|history| history.event) };

    let is_valid = FriendshipEvent::validate_new_event_is_valid(&last_event, room_event);
    if !is_valid {
        return Err(SynapseError::InvalidEvent);
    }

    match room_event {
        FriendshipEvent::REQUEST => {
            calculate_new_friendship_status(acting_user, last_history, room_event)
        }
        FriendshipEvent::ACCEPT => {
            calculate_new_friendship_status(acting_user, last_history, room_event)
        }
        FriendshipEvent::CANCEL => {
            if let Some(last_history) = last_history {
                if last_history.acting_user.eq_ignore_ascii_case(acting_user) {
                    return Ok(FriendshipStatus::NotFriends);
                }
            }

            Err(SynapseError::InvalidEvent)
        }
        FriendshipEvent::REJECT => {
            if let Some(last_history) = last_history {
                if !last_history.acting_user.eq_ignore_ascii_case(acting_user) {
                    return Ok(FriendshipStatus::NotFriends);
                }
            }

            Err(SynapseError::InvalidEvent)
        }
        FriendshipEvent::DELETE => Ok(FriendshipStatus::NotFriends),
    }
}

// This function assumes that the room event is  valid for the last event
fn calculate_new_friendship_status(
    acting_user: &str,
    last_history: &Option<FriendshipHistory>,
    room_event: FriendshipEvent,
) -> Result<FriendshipStatus, SynapseError> {
    if last_history.is_none() {
        return match room_event {
            FriendshipEvent::REQUEST => Ok(FriendshipStatus::Requested(acting_user.to_string())),
            _ => Err(SynapseError::InvalidEvent),
        };
    }

    let last_history = last_history.as_ref().unwrap();

    match last_history.event {
        FriendshipEvent::REQUEST => {
            // since the room event should only be accept or request it can only be done by the second user
            if last_history.acting_user.eq_ignore_ascii_case(acting_user) {
                return Err(SynapseError::InvalidEvent);
            }

            match room_event {
                FriendshipEvent::ACCEPT => Ok(FriendshipStatus::Friends),
                _ => Err(SynapseError::InvalidEvent),
            }
        }
        FriendshipEvent::ACCEPT => Err(SynapseError::InvalidEvent),
        _ => match room_event {
            FriendshipEvent::REQUEST => Ok(FriendshipStatus::Requested(acting_user.to_string())),
            _ => Err(SynapseError::InvalidEvent),
        },
    }
}

async fn update_friendship_status(
    friendship: &Option<Friendship>,
    acting_user: &str,
    second_user: &str,
    current_status: FriendshipStatus,
    new_status: FriendshipStatus,
    room_event: FriendshipEvent,
    db: &DatabaseComponent,
    friendships_repository: &FriendshipsRepository,
    friendship_history_repository: &FriendshipHistoryRepository,
) -> Result<(), SynapseError> {
    // The only case where we don't create the friendship if it didn't exist
    // If they are still no friends, it's unnecessary to create a friendship
    if current_status == new_status {
        return Ok(());
    }

    let transaction = db.start_transaction().await;

    if transaction.is_err() {
        let err = transaction.err().unwrap();
        log::error!("Couldn't start transaction to store friendship update {err}");
        return Err(SynapseError::CommonError(CommonError::Unknown));
    }

    // start transaction
    let transaction = transaction.unwrap();

    // store friendship update
    let is_active = new_status == FriendshipStatus::Friends;
    let (friendship_id_result, transaction) = store_friendship_update(
        friendship,
        is_active,
        acting_user,
        second_user,
        friendships_repository,
        transaction,
    )
    .await;

    let friendship_id = match friendship_id_result {
        Ok(friendship_id) => friendship_id,
        Err(err) => {
            log::error!("Couldn't store friendship update {err}");
            let _ = transaction.rollback().await;

            return Err(SynapseError::CommonError(CommonError::Unknown));
        }
    };

    let room_event = serde_json::to_string(&room_event).unwrap();

    // store history
    let (friendship_history_result, transaction) = friendship_history_repository
        .create(
            friendship_id,
            room_event.as_str(),
            acting_user,
            None,
            Some(transaction),
        )
        .await;

    let transaction = transaction.unwrap();

    if friendship_history_result.is_err() {
        let err = friendship_history_result.unwrap_err();
        log::error!("Couldn't store friendship history update {err}");
        let _ = transaction.rollback().await;

        return Err(SynapseError::CommonError(CommonError::Unknown));
    }

    // end transaction
    let transaction_result = transaction.commit().await;

    transaction_result.map_err(|err| {
        log::error!("Couldn't commit transaction to store friendship update {err}");
        SynapseError::CommonError(CommonError::Unknown)
    })
}

async fn store_friendship_update<'a>(
    friendship: &'a Option<Friendship>,
    is_active: bool,
    address_0: &'a str,
    address_1: &'a str,
    friendships_repository: &'a FriendshipsRepository,
    transaction: Transaction<'a, Postgres>,
) -> (Result<Uuid, SynapseError>, Transaction<'a, Postgres>) {
    match friendship {
        Some(friendship) => {
            let (res, transaction) = friendships_repository
                .update_friendship_status(&friendship.id, is_active, Some(transaction))
                .await;

            let res = match res {
                Ok(_) => Ok(friendship.id),
                Err(err) => {
                    log::warn!("Couldn't update friendship {err}");
                    Err(SynapseError::CommonError(CommonError::Unknown))
                }
            };

            (res, transaction.unwrap())
        }
        None => {
            let (friendship_id, transaction) = friendships_repository
                .create_new_friendships((address_0, address_1), Some(transaction))
                .await;
            (
                friendship_id.map_err(|err| {
                    log::warn!("Couldn't crate new friendship {err}");
                    SynapseError::CommonError(CommonError::Unknown)
                }),
                transaction.unwrap(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use crate::{
        entities::friendship_history::FriendshipHistory, routes::synapse::errors::SynapseError,
    };

    use super::{process_friendship_status, FriendshipEvent, FriendshipStatus};

    fn get_last_history(event: FriendshipEvent, acting_user: &str) -> Option<FriendshipHistory> {
        Some(FriendshipHistory {
            event,
            acting_user: acting_user.to_string(),
            friendship_id: uuid::uuid!("223ab239-a69f-40e5-9932-d92348a43fd0"),
            metadata: None,
            timestamp: NaiveDate::from_ymd_opt(2023, 1, 2)
                .unwrap()
                .and_hms_nano_opt(10, 1, 1, 0)
                .unwrap(),
        })
    }

    #[test]
    fn test_process_friendship_status_not_friends_requested() {
        let acting_user = "user";
        let last_history = None;
        let event = FriendshipEvent::REQUEST;
        let res = process_friendship_status(acting_user, &last_history, event);

        assert_eq!(
            res,
            Ok(FriendshipStatus::Requested(acting_user.to_string()))
        );
    }

    #[test]
    fn test_process_friendship_status_requested_accepted() {
        let acting_user = "user";
        let event = FriendshipEvent::ACCEPT;
        let last_history = get_last_history(FriendshipEvent::REQUEST, "another user");
        let res = process_friendship_status(acting_user, &last_history, event);

        assert_eq!(res, Ok(FriendshipStatus::Friends));
    }

    #[test]
    fn test_process_friendship_status_requested_rejected() {
        let acting_user = "user";
        let event = FriendshipEvent::REJECT;
        let last_history = get_last_history(FriendshipEvent::REQUEST, "another user");
        let res = process_friendship_status(acting_user, &last_history, event);

        assert_eq!(res, Ok(FriendshipStatus::NotFriends));
    }

    #[test]
    fn test_process_friendship_status_requested_accepted_same_user_should_err() {
        let acting_user = "user";
        let event = FriendshipEvent::ACCEPT;
        let last_history = get_last_history(FriendshipEvent::REQUEST, acting_user);
        let res = process_friendship_status(acting_user, &last_history, event);

        assert_eq!(res, Err(SynapseError::InvalidEvent));
    }

    #[test]
    fn test_process_friendship_status_requested_requested_same_user_should_err() {
        let acting_user = "user";
        let event = FriendshipEvent::REQUEST;
        let last_history = get_last_history(event, acting_user);
        let res = process_friendship_status(acting_user, &last_history, event);

        assert_eq!(res, Err(SynapseError::InvalidEvent));
    }

    #[test]
    fn test_process_friendship_status_requested_rejected_same_user_should_err() {
        let acting_user = "user";
        let event = FriendshipEvent::REJECT;
        let last_history = get_last_history(FriendshipEvent::REQUEST, acting_user);
        let res = process_friendship_status(acting_user, &last_history, event);

        assert_eq!(res, Err(SynapseError::InvalidEvent));
    }

    #[test]
    fn test_process_friendship_status_friends_remove() {
        let acting_user = "user";
        let event = FriendshipEvent::DELETE;
        let last_history = get_last_history(FriendshipEvent::ACCEPT, "another user");

        let res = process_friendship_status(acting_user, &last_history, event);

        assert_eq!(res, Ok(FriendshipStatus::NotFriends));
    }

    #[test]
    fn test_process_friendship_status_requested_cancel() {
        let acting_user = "user";
        let event = FriendshipEvent::CANCEL;
        let last_history = get_last_history(FriendshipEvent::REQUEST, acting_user);
        let res = process_friendship_status(acting_user, &last_history, event);

        assert_eq!(res, Ok(FriendshipStatus::NotFriends));
    }

    #[test]
    fn test_process_friendship_status_requested_cancel_from_another_user_should_err() {
        let acting_user = "user";
        let event = FriendshipEvent::CANCEL;
        let last_history = get_last_history(FriendshipEvent::REQUEST, "another user");
        let res = process_friendship_status(acting_user, &last_history, event);

        assert_eq!(res, Err(SynapseError::InvalidEvent));
    }

    #[test]
    fn test_process_friendship_status_requested_reject_from_same_user_should_err() {
        let acting_user = "user";
        let event = FriendshipEvent::REJECT;
        let last_history = get_last_history(FriendshipEvent::REQUEST, acting_user);
        let res = process_friendship_status(acting_user, &last_history, event);

        assert_eq!(res, Err(SynapseError::InvalidEvent));
    }

    #[test]
    fn test_process_friendship_status_requested_requested_should_not_become_friends() {
        let acting_user = "user";
        let event = FriendshipEvent::REQUEST;
        let last_history = get_last_history(FriendshipEvent::REQUEST, "another user");
        let res = process_friendship_status(acting_user, &last_history, event);

        assert_eq!(res, Err(SynapseError::InvalidEvent));
    }

    #[test]
    fn test_process_friendship_status_friends_accept_should_err_invalid_event() {
        let acting_user = "user";
        let event = FriendshipEvent::REQUEST;
        let last_history = get_last_history(FriendshipEvent::ACCEPT, "another user");
        let res = process_friendship_status(acting_user, &last_history, event);

        assert_eq!(res, Err(SynapseError::InvalidEvent));
    }
}
