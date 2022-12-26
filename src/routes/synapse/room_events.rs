use actix_web::{
    put,
    web::{self, Data},
    HttpMessage, HttpRequest, HttpResponse,
};
use serde::{Deserialize, Serialize};

use crate::{
    components::{
        app::AppComponents,
        database::{DBRepositories, DatabaseComponent},
        synapse::SynapseComponent,
    },
    entities::{
        friendship_history::{self, FriendshipHistory},
        friendships::Friendship,
    },
    middlewares::check_auth::{Token, UserId},
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

pub enum FriendshipStatus {
    Friends,
    Requested(String),
    Rejected,
    NotFriends,
}

impl PartialEq for FriendshipStatus {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

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
    let mut members: Vec<String> = vec![];
    match members_result {
        Ok(response) => {
            members = response
                .chunk
                .iter()
                .map(|member| member.user_id.clone())
                .collect::<Vec<String>>()
        }
        Err(err) => return Err(SynapseError::CommonError(err)),
    }

    if members.len() != 2 {
        return Err(SynapseError::FriendshipNotFound);
    }

    // GET LAST STATUS FROM DB
    let repos = db.get_repos().as_ref().unwrap();

    let friendship =
        get_friendship_from_db(repos, (members.get(0).unwrap(), members.get(1).unwrap())).await?;

    let current_status = get_friendship_status_from_db(friendship, repos).await?;

    // PROCESS NEW STATUS OF FRIENDSHIP
    let new_status = process_friendship_status(acting_user.to_string(), current_status, room_event);

    // UPDATE FRIENDSHIP IN DB

    let res = synapse.store_room_event(token, room_id, room_event).await;

    match res {
        Ok(res) => Ok(res),
        Err(err) => return Err(SynapseError::CommonError(err)),
    }
}

async fn get_friendship_from_db(
    repos: &DBRepositories,
    members: (&String, &String),
) -> Result<Option<Friendship>, SynapseError> {
    let friendship_result = repos.get_friendships().get((&members.0, &members.1)).await;

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
    friendship: Option<Friendship>,
    repos: &DBRepositories,
) -> Result<FriendshipStatus, SynapseError> {
    if friendship.is_none() {
        return Ok(FriendshipStatus::NotFriends);
    }

    let friendship = friendship.unwrap();

    let friendship_history_result = repos.get_friendship_history().get(friendship.id).await;

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
    current_status: FriendshipStatus,
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
    current_status: FriendshipStatus,
    room_event: FriendshipEvent,
) -> FriendshipStatus {
    // if someone accepts or requests a friendship without an existing or a new one, the status shouldn't change
    if current_status != FriendshipStatus::Requested("".to_string())
        && room_event != FriendshipEvent::REQUEST
    {
        return current_status;
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
        _ => {
            if room_event == FriendshipEvent::REQUEST {
                return FriendshipStatus::Requested(acting_user);
            }

            FriendshipStatus::NotFriends
        }
    }
}
