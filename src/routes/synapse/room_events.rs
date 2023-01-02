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
    event_id: String,
}

#[derive(Deserialize, Serialize)]
pub struct RoomEventRequestBody {
    pub r#type: FriendshipEvent,
}

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, Copy)]
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

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum FriendshipStatus {
    Friends,
    Requested(String),
    Rejected,
    NotFriends,
}

// impl PartialEq for FriendshipStatus {
//     fn eq(&self, other: &Self) -> bool {
//         let discriminant = core::mem::discriminant(other);

//         core::mem::discriminant(self) == core::mem::discriminant(other)
//     }
// }

impl FriendshipStatus {
    fn from_str(str: String, owner: String) -> Self {
        let friendship_event = serde_json::from_str::<FriendshipEvent>(&str);

        if friendship_event.is_err() {
            log::error!("Invalid friendship event stored in database {}", str);
            return FriendshipStatus::NotFriends;
        }

        let friendship_event = friendship_event.unwrap();

        match friendship_event {
            FriendshipEvent::REQUEST => FriendshipStatus::Requested(owner),
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
    let extensions = req.extensions();
    let logged_in_user = extensions.get::<UserId>().unwrap().0.as_str();
    let token = extensions.get::<Token>().unwrap().0.as_str();

    println!("Acting user: {logged_in_user}");

    let response = process_room_event(
        logged_in_user,
        token,
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

    return Err(err);
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

    // GET LAST STATUS FROM DB
    let repos = db.db_repos.as_ref().unwrap();

    let friendship = get_friendship_from_db(&repos.friendships, &address_0, &address_1).await?;

    let current_status =
        get_friendship_status_from_db(&friendship, &repos.friendship_history).await?;

    // PROCESS NEW STATUS OF FRIENDSHIP
    let new_status =
        process_friendship_status(acting_user.to_string(), &current_status, room_event);

    let second_user = if address_0.eq_ignore_ascii_case(acting_user) {
        address_0
    } else {
        address_1
    };

    // UPDATE FRIENDSHIP ACCORDINGLY IN DB
    update_friendship_status(
        &friendship,
        &acting_user,
        &second_user,
        current_status,
        new_status,
        room_event,
        &db,
        &repos.friendships,
        &repos.friendship_history,
    )
    .await?;

    let res = synapse.store_room_event(token, room_id, room_event).await;

    match res {
        Ok(res) => Ok(res),
        Err(err) => return Err(SynapseError::CommonError(err)),
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
        Err(err) => return Err(SynapseError::CommonError(err)),
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

async fn get_friendship_status_from_db(
    friendship: &Option<Friendship>,
    friendship_history_repository: &FriendshipHistoryRepository,
) -> Result<FriendshipStatus, SynapseError> {
    let friendship = {
        match friendship {
            Some(friendship) => friendship,
            None => return Ok(FriendshipStatus::NotFriends),
        }
    };

    let (friendship_history_result, _) =
        friendship_history_repository.get(friendship.id, None).await;

    if friendship_history_result.is_err() {
        let err = friendship_history_result.err().unwrap();

        log::warn!("Error getting friendship history in room event {}", err);
        return Err(SynapseError::CommonError(
            crate::routes::v1::error::CommonError::Unknown,
        ));
    }

    let friendship_history = friendship_history_result.unwrap();

    Ok(calculate_current_friendship_status(friendship_history))
}

fn calculate_current_friendship_status(
    friendship_history: Option<FriendshipHistory>,
) -> FriendshipStatus {
    if friendship_history.is_none() {
        return FriendshipStatus::NotFriends;
    }

    let friendship_history = friendship_history.unwrap();

    FriendshipStatus::from_str(friendship_history.event, friendship_history.acting_user)
}

fn process_friendship_status(
    acting_user: String,
    current_status: &FriendshipStatus,
    room_event: FriendshipEvent,
) -> FriendshipStatus {
    match room_event {
        FriendshipEvent::REQUEST => verify_if_friends(acting_user, current_status, room_event),
        FriendshipEvent::CANCEL => FriendshipStatus::NotFriends,
        FriendshipEvent::ACCEPT => verify_if_friends(acting_user, current_status, room_event),
        FriendshipEvent::REJECT => FriendshipStatus::NotFriends,
        FriendshipEvent::DELETE => FriendshipStatus::NotFriends,
    }
}

fn verify_if_friends(
    acting_user: String,
    current_status: &FriendshipStatus,
    room_event: FriendshipEvent,
) -> FriendshipStatus {
    // if someone accepts or requests a friendship without an existing or a new one, the status shouldn't change
    if !matches!(*current_status, FriendshipStatus::Requested(_))
        && room_event != FriendshipEvent::REQUEST
    {
        return (*current_status).clone();
    }

    match current_status {
        FriendshipStatus::Requested(old_request) => {
            if old_request.eq_ignore_ascii_case(&acting_user) {
                return FriendshipStatus::Requested(acting_user);
            }

            match room_event {
                FriendshipEvent::ACCEPT => FriendshipStatus::Friends,
                FriendshipEvent::REQUEST => FriendshipStatus::Friends,
                FriendshipEvent::CANCEL => FriendshipStatus::NotFriends,
                FriendshipEvent::REJECT => FriendshipStatus::NotFriends,
                FriendshipEvent::DELETE => FriendshipStatus::NotFriends,
            }
        }
        FriendshipStatus::Friends => FriendshipStatus::Friends,
        _ => {
            if room_event == FriendshipEvent::REQUEST {
                return FriendshipStatus::Requested(acting_user);
            }

            FriendshipStatus::NotFriends
        }
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

    use super::{process_friendship_status, FriendshipEvent, FriendshipStatus};

    #[test]
    fn test_process_friendship_status_not_friends_requested() {
        let acting_user = "user";
        let current_status = FriendshipStatus::NotFriends;
        let event = FriendshipEvent::REQUEST;
        let res = process_friendship_status(acting_user.to_string(), &current_status, event);

        assert_eq!(res, FriendshipStatus::Requested(acting_user.to_string()));
    }

    #[test]
    fn test_process_friendship_status_requested_accepted() {
        let acting_user = "user";
        let current_status = FriendshipStatus::Requested("another user".to_string());
        let event = FriendshipEvent::ACCEPT;
        let res = process_friendship_status(acting_user.to_string(), &current_status, event);

        assert_eq!(res, FriendshipStatus::Friends);
    }

    #[test]
    fn test_process_friendship_status_requested_rejected() {
        let acting_user = "user";
        let current_status = FriendshipStatus::Requested("another user".to_string());
        let event = FriendshipEvent::REJECT;
        let res = process_friendship_status(acting_user.to_string(), &current_status, event);

        assert_eq!(res, FriendshipStatus::NotFriends);
    }

    #[test]
    fn test_process_friendship_status_requested_accepted_same_user() {
        let acting_user = "user";
        let current_status = FriendshipStatus::Requested(acting_user.to_string());
        let event = FriendshipEvent::ACCEPT;
        let res = process_friendship_status(acting_user.to_string(), &current_status, event);

        assert_eq!(res, current_status);
    }

    #[test]
    fn test_process_friendship_status_requested_requested_same_user() {
        let acting_user = "user";
        let current_status = FriendshipStatus::Requested(acting_user.to_string());
        let event = FriendshipEvent::REQUEST;
        let res = process_friendship_status(acting_user.to_string(), &current_status, event);

        assert_eq!(res, current_status);
    }

    #[test]
    fn test_process_friendship_status_requested_rejected_same_user_should_remove() {
        let acting_user = "user";
        let current_status = FriendshipStatus::Requested(acting_user.to_string());
        let event = FriendshipEvent::REJECT;
        let res = process_friendship_status(acting_user.to_string(), &current_status, event);

        assert_eq!(res, FriendshipStatus::NotFriends);
    }

    #[test]
    fn test_process_friendship_status_friends_remove() {
        let acting_user = "user";
        let current_status = FriendshipStatus::Friends;
        let event = FriendshipEvent::DELETE;
        let res = process_friendship_status(acting_user.to_string(), &current_status, event);

        assert_eq!(res, FriendshipStatus::NotFriends);
    }

    #[test]
    fn test_process_friendship_status_requested_cancel() {
        let acting_user = "user";
        let current_status = FriendshipStatus::Friends;
        let event = FriendshipEvent::CANCEL;
        let res = process_friendship_status(acting_user.to_string(), &current_status, event);

        assert_eq!(res, FriendshipStatus::NotFriends);
    }

    #[test]
    fn test_process_friendship_status_requested_requested_should_become_friends() {
        let acting_user = "user";
        let current_status = FriendshipStatus::Requested("another user".to_string());
        let event = FriendshipEvent::REQUEST;
        let res = process_friendship_status(acting_user.to_string(), &current_status, event);

        assert_eq!(res, FriendshipStatus::Friends);
    }

    #[test]
    fn test_process_friendship_status_friends_accept_should_stay_friends() {
        let acting_user = "user";
        let current_status = FriendshipStatus::Friends;
        let event = FriendshipEvent::REQUEST;
        let res = process_friendship_status(acting_user.to_string(), &current_status, event);

        assert_eq!(res, FriendshipStatus::Friends);
    }
}
