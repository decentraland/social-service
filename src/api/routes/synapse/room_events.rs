use actix_web::{
    put,
    web::{self, Data},
    HttpMessage, HttpRequest, HttpResponse,
};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    api::middlewares::check_auth::Token,
    components::{
        app::AppComponents,
        database::{DatabaseComponent, DatabaseComponentImplementation},
        synapse::{RoomMembersResponse, SynapseComponent},
        users_cache::UserId,
    },
    domain::{
        error::CommonError, friendship_event::FriendshipEvent, friendship_status::FriendshipStatus,
    },
    entities::{
        friendship_history::{FriendshipHistory, FriendshipHistoryRepository, FriendshipMetadata},
        friendships::{Friendship, FriendshipRepositoryImplementation, FriendshipsRepository},
    },
};

use super::errors::SynapseError;

#[derive(Deserialize, Serialize)]
pub struct RoomEventResponse {
    pub event_id: String,
}

#[derive(Deserialize, Serialize)]
pub struct RoomJoinResponse {
    pub room_id: String,
}

#[derive(Deserialize, Serialize)]
pub struct JoinedRoomsResponse {
    pub joined_rooms: Vec<String>,
}

#[derive(Deserialize, Serialize)]
pub struct RoomEventRequestBody {
    pub r#type: FriendshipEvent,
    pub message: Option<String>,
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
            .clone();

        let token = extensions
            .get::<Token>()
            .expect("To have an authentication token")
            .0
            .clone();

        (logged_in_user, token)
    };

    let room_message_body = body.message.as_deref();

    let response = process_room_event(
        &logged_in_user.social_id,
        &token,
        room_id.as_str(),
        body.r#type,
        room_message_body,
        &app_data.db,
        &app_data.synapse,
    )
    .await;

    match response {
        Ok(res) => Ok(HttpResponse::Ok().json(res)),
        Err(err) => Err(err),
    }
}

async fn process_room_event<'a>(
    acting_user: &str,
    token: &str,
    room_id: &str,
    room_event: FriendshipEvent,
    room_message_body: Option<&str>,
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

    let room_info = RoomInfo {
        room_event,
        room_message_body,
        room_id,
    };
    let friendship_ports = FriendshipPorts {
        db,
        friendships_repository: &repos.friendships,
        friendship_history_repository: &repos.friendship_history,
    };

    // If the status has not changed, no action is taken.
    if current_status == new_status {
        return Ok(RoomEventResponse {
            event_id: room_id.to_string(),
        });
    }

    // Start transaction
    let transaction = match friendship_ports.db.start_transaction().await {
        Ok(tx) => tx,
        Err(error) => {
            log::error!("Couldn't start transaction to store friendship update {error}");
            return Err(SynapseError::CommonError(CommonError::Unknown(
                "".to_owned(),
            )));
        }
    };

    // UPDATE FRIENDSHIP ACCORDINGLY IN DB
    let transaction = update_friendship_status(
        &friendship,
        acting_user,
        &second_user,
        new_status,
        room_info,
        friendship_ports,
        transaction,
    )
    .await?;

    // If it's a friendship request event and the request contains a message, we send a message event to the given room.
    store_message_in_synapse_room(token, room_id, room_event, room_message_body, synapse).await?;

    // Store friendship event in the given room
    let res = synapse
        .store_room_event(token, room_id, room_event, room_message_body)
        .await;

    match res {
        Ok(value) => {
            // End transaction
            let transaction_result = transaction.commit().await;

            match transaction_result {
                Ok(_) => Ok(value),
                Err(_) => Err(SynapseError::CommonError(CommonError::Unknown(
                    "".to_owned(),
                ))),
            }
        }
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
                .map(|member| match member.social_user_id.clone() {
                    Some(social_user_id) => social_user_id,
                    None => "".to_string(),
                })
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
        return Err(SynapseError::CommonError(CommonError::Unknown(
            "".to_owned(),
        )));
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
            crate::domain::error::CommonError::Unknown("".to_owned()),
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

pub struct RoomInfo<'a> {
    room_event: FriendshipEvent,
    room_message_body: Option<&'a str>,
    room_id: &'a str,
}

pub struct FriendshipPorts<'a> {
    db: &'a DatabaseComponent,
    friendships_repository: &'a FriendshipsRepository,
    friendship_history_repository: &'a FriendshipHistoryRepository,
}

async fn update_friendship_status<'a>(
    friendship: &'a Option<Friendship>,
    acting_user: &'a str,
    second_user: &'a str,
    new_status: FriendshipStatus,
    room_info: RoomInfo<'a>,
    friendship_ports: FriendshipPorts<'a>,
    transaction: Transaction<'static, Postgres>,
) -> Result<Transaction<'static, Postgres>, SynapseError> {
    // store friendship update
    let is_active = new_status == FriendshipStatus::Friends;
    let (friendship_id_result, transaction) = store_friendship_update(
        friendship,
        is_active,
        acting_user,
        second_user,
        room_info.room_id,
        friendship_ports.friendships_repository,
        transaction,
    )
    .await;

    let friendship_id = match friendship_id_result {
        Ok(friendship_id) => friendship_id,
        Err(err) => {
            log::error!("Couldn't store friendship update {err}");
            let _ = transaction.rollback().await;

            return Err(SynapseError::CommonError(CommonError::Unknown(
                "".to_owned(),
            )));
        }
    };

    let room_event = match serde_json::to_string(&room_info.room_event) {
        Ok(room_event_string) => room_event_string,
        Err(err) => {
            log::error!("Error serializing room event: {:?}", err);
            let _ = transaction.rollback().await;
            return Err(SynapseError::CommonError(CommonError::Unknown(
                "".to_owned(),
            )));
        }
    };

    let metadata = sqlx::types::Json(FriendshipMetadata {
        message: room_info.room_message_body.map(|m| m.to_string()),
        synapse_room_id: Some(room_info.room_id.to_string()),
        migrated_from_synapse: None,
    });

    // store history
    let (friendship_history_result, transaction) = friendship_ports
        .friendship_history_repository
        .create(
            friendship_id,
            &room_event,
            acting_user,
            Some(metadata),
            Some(transaction),
        )
        .await;

    let transaction = transaction.unwrap();

    match friendship_history_result {
        Ok(_) => Ok(transaction),
        Err(err) => {
            log::error!("Couldn't store friendship history update: {:?}", err);
            let _ = transaction.rollback().await;
            Err(SynapseError::CommonError(CommonError::Unknown(
                "".to_owned(),
            )))
        }
    }
}

async fn store_friendship_update(
    friendship: &Option<Friendship>,
    is_active: bool,
    address_0: &str,
    address_1: &str,
    synapse_room_id: &str,
    friendships_repository: &FriendshipsRepository,
    transaction: Transaction<'static, Postgres>,
) -> (Result<Uuid, SynapseError>, Transaction<'static, Postgres>) {
    match friendship {
        Some(friendship) => {
            let (res, transaction) = friendships_repository
                .update_friendship_status(&friendship.id, is_active, Some(transaction))
                .await;

            let res = match res {
                Ok(_) => Ok(friendship.id),
                Err(err) => {
                    log::warn!("Couldn't update friendship {err}");
                    Err(SynapseError::CommonError(CommonError::Unknown(
                        "".to_owned(),
                    )))
                }
            };

            (res, transaction.unwrap())
        }
        None => {
            let (friendship_id, transaction) = friendships_repository
                .create_new_friendships(
                    (address_0, address_1),
                    false,
                    synapse_room_id,
                    Some(transaction),
                )
                .await;
            (
                friendship_id.map_err(|err| {
                    log::warn!("Couldn't crate new friendship {err}");
                    SynapseError::CommonError(CommonError::Unknown("".to_owned()))
                }),
                transaction.unwrap(),
            )
        }
    }
}

/// If it's a friendship request event and the request contains a message, we send a message event to the given room.
async fn store_message_in_synapse_room<'a>(
    token: &str,
    room_id: &str,
    room_event: FriendshipEvent,
    room_message_body: Option<&str>,
    synapse: &SynapseComponent,
) -> Result<(), SynapseError> {
    // Check if it's a `request` event.
    if room_event != FriendshipEvent::REQUEST {
        return Ok(());
    }

    // Check if there is a message, if any, send the message event to the given room.
    if let Some(val) = room_message_body {
        // Check if the message body is not empty
        if !val.is_empty() {
            for retry_count in 0..3 {
                match synapse
                    .send_message_event_given_room(token, room_id, room_event, val)
                    .await
                {
                    Ok(_) => {
                        return Ok(());
                    }
                    Err(err) => {
                        if retry_count == 2 {
                            log::error!("[RPC] Store message in synapse room > Error {err}");
                            return Err(SynapseError::CommonError(err));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use crate::{
        api::routes::synapse::errors::SynapseError,
        domain::{friendship_event::FriendshipEvent, friendship_status::FriendshipStatus},
        entities::friendship_history::FriendshipHistory,
    };

    use super::process_friendship_status;

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
