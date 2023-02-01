use actix_http::HttpMessage;
use actix_web::{
    get,
    web::{self, Data},
    HttpRequest, HttpResponse,
};

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    components::app::AppComponents,
    entities::{
        friendship_history::FriendshipHistory, friendships::FriendshipRepositoryImplementation,
    },
    middlewares::check_auth::UserId,
    routes::v1::{
        error::CommonError,
        friendships::{errors::FriendshipsError, types::MessageRequestEventResponse},
    },
};

use super::types::MessageRequestEvent;

#[derive(Deserialize, Serialize)]
pub struct RequestEventParam {
    pub timestamp_from: i64, // timestamp in milis
    pub timestamp_to: i64,   // timestamp in milis
}

#[get("/v1/friendships/{friendshipId}/request-events/messages")]
async fn get_sent_messages_request_event(
    req: HttpRequest,
    param: web::Query<RequestEventParam>,
    friendship_id: web::Path<Uuid>,
    app_data: Data<AppComponents>,
) -> Result<HttpResponse, FriendshipsError> {
    let logged_in_user: UserId = req
        .extensions()
        .get::<UserId>()
        .expect("to have a UserId")
        .clone();

    // Retrieve users from friendship and verify permissions.
    let users = get_users_friendship(app_data.clone(), *friendship_id).await;
    if !users.is_empty() {
        has_permission(logged_in_user.social_id.as_str(), &users[0], &users[1]);
    } else {
        return Err(FriendshipsError::CommonError(CommonError::BadRequest(
            format!("You don't have permission to view the sent request messages for friendship {friendship_id}"),
        )));
    }

    // Convert it to a timestamp type that can be understood by PostgreSQL.
    let timestamp_from_naive = match NaiveDateTime::from_timestamp_opt(param.timestamp_from, 0) {
        Some(val) => val,
        None => {
            return Err(FriendshipsError::CommonError(CommonError::BadRequest(
                "Failed to convert timestamp_to to NaiveDateTime".to_string(),
            )))
        }
    };

    let timestamp_to_naive = match NaiveDateTime::from_timestamp_opt(param.timestamp_to, 0) {
        Some(val) => val,
        None => {
            return Err(FriendshipsError::CommonError(CommonError::BadRequest(
                "Failed to convert timestamp_to to NaiveDateTime".to_string(),
            )))
        }
    };

    // Get the history of friendship request events.
    match &app_data.db.db_repos {
        Some(repos) => {
            let (history, _) = repos
                .friendship_history
                .get_friendship_request_event_history(
                    *friendship_id,
                    timestamp_from_naive,
                    timestamp_to_naive,
                    true,
                    None,
                )
                .await;
            match history {
                Err(_) => Err(FriendshipsError::CommonError(CommonError::Unknown)),
                Ok(history) => {
                    // Get request events with non-empty messages.
                    let response =
                        MessageRequestEventResponse::new(get_request_events_with_messages(history));
                    Ok(HttpResponse::Ok().json(response))
                }
            }
        }
        None => Err(FriendshipsError::CommonError(CommonError::NotFound)),
    }
}

/// Filters the `friendship_history`. It checks if the `metadata` of each `FriendshipHistory` contains the key `message_body`
/// and if it contains a non-empty value. If the check returns true, the struct is included in the filtered vector.
fn get_request_events_with_messages(
    friendship_history: Vec<FriendshipHistory>,
) -> Vec<MessageRequestEvent> {
    friendship_history
        .into_iter()
        .filter_map(|history| {
            history
                .metadata
                .as_ref()
                .and_then(|meta| meta.get("message_body"))
                .filter(|s| !s.is_empty())
                .map(|value| MessageRequestEvent {
                    friendship_id: history.friendship_id.to_string(),
                    acting_user: history.acting_user.to_string(),
                    timestamp: history.timestamp.timestamp(),
                    body: value.to_string(),
                })
        })
        .collect()
}

/// Retrieve users from friendship
async fn get_users_friendship(app_data: Data<AppComponents>, friendship_id: Uuid) -> Vec<String> {
    match &app_data.db.db_repos {
        Some(repos) => {
            let (result, _) = repos
                .friendships
                .get_users_from_friendship(&friendship_id, None)
                .await;
            match result {
                Ok(users) => users.unwrap_or(vec![]),
                Err(_) => return vec![],
            }
        }
        None => return vec![],
    }
}

///
fn has_permission(logged_user_id: &str, user_id_1: &str, user_id_2: &str) -> bool {
    logged_user_id.eq_ignore_ascii_case(user_id_1) || logged_user_id.eq_ignore_ascii_case(user_id_2)
}
