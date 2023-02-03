use actix_http::HttpMessage;
use actix_web::{
    get,
    web::{self, Data},
    HttpRequest, HttpResponse,
};

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

#[get("/v1/friendships/{friendshipId}/request-events/messages")]
async fn get_sent_messages_request_event(
    req: HttpRequest,
    friendship_id: web::Path<Uuid>,
    app_data: Data<AppComponents>,
) -> Result<HttpResponse, FriendshipsError> {
    let logged_in_user: UserId = req
        .extensions()
        .get::<UserId>()
        .expect("to have a UserId")
        .clone();

    // Retrieve users from friendship.
    let users = match get_users_friendship(app_data.clone(), *friendship_id).await {
        Some(value) => value,
        None => return Err(FriendshipsError::CommonError(CommonError::NotFound)),
    };

    // Verify that the logged in user is present in the friendship.
    if !has_permission(logged_in_user.social_id.as_str(), users) {
        return Err(FriendshipsError::CommonError(CommonError::Unauthorized));
    }

    // Get the history of friendship request events.
    match &app_data.db.db_repos {
        Some(repos) => {
            let (history, _) = repos
                .friendship_history
                .get_friendship_request_event_history(*friendship_id, None)
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

/// Filters the `friendship_history`. It checks if the `metadata` of each `FriendshipHistory` contains the key `message`
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
                .and_then(|meta| meta.message.clone())
                .filter(|s| !s.is_empty())
                .map(|value| MessageRequestEvent {
                    friendship_id: history.friendship_id.to_string(),
                    acting_user: history.acting_user.to_string(),
                    timestamp: history.timestamp.timestamp(),
                    message: value,
                })
        })
        .collect()
}

/// Retrieve users from friendship.
async fn get_users_friendship(
    app_data: Data<AppComponents>,
    friendship_id: Uuid,
) -> Option<Vec<String>> {
    match &app_data.db.db_repos {
        Some(repos) => {
            let (result, _) = repos
                .friendships
                .get_users_from_friendship(&friendship_id, None)
                .await;
            match result {
                Ok(users) => users,
                Err(_) => None,
            }
        }
        None => None,
    }
}

/// Check if the logged-in user is part of the friendship.
fn has_permission(logged_user_id: &str, users: Vec<String>) -> bool {
    users
        .iter()
        .any(|user| logged_user_id.eq_ignore_ascii_case(user))
}
