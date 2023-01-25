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
    entities::friendship_history::FriendshipHistory,
    middlewares::check_auth::UserId,
    routes::v1::{
        error::CommonError,
        friendships::{errors::FriendshipsError, types::MessageRequestEventResponse},
    },
};

use super::types::MessageRequestEvent;

#[derive(Deserialize, Serialize)]
pub struct RequestEventRequestBody {
    pub timestamp_from: i64, // timestamp in milis
    pub timestamp_to: i64,   // timestamp in milis
}

#[get("/v1/friendships/{friendshipId}/request-events/messages")]
async fn get_sent_messages_request_event(
    req: HttpRequest,
    body: web::Json<RequestEventRequestBody>,
    friendship_id: web::Path<String>,
    app_data: Data<AppComponents>,
) -> Result<HttpResponse, FriendshipsError> {
    let _logged_in_user = req
        .extensions()
        .get::<UserId>()
        .expect("to have a UserId")
        .clone();

    // Convert it to a timestamp type that can be understood by PostgreSQL.
    let timestamp_from_naive: NaiveDateTime =
        NaiveDateTime::from_timestamp_opt(body.timestamp_from, 0).unwrap();
    let timestamp_to_naive: NaiveDateTime =
        NaiveDateTime::from_timestamp_opt(body.timestamp_to, 0).unwrap();

    // Get the history of friendship request events.
    match &app_data.db.db_repos {
        Some(repos) => {
            let (history, _) = repos
                .friendship_history
                .get_friendship_request_event_history(
                    friendship_id.parse::<Uuid>().unwrap(),
                    timestamp_from_naive,
                    timestamp_to_naive,
                    None,
                )
                .await;
            match history {
                Err(_) => Err(FriendshipsError::CommonError(CommonError::Unknown)),
                Ok(history) => {
                    // Get request events with messages
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
        .iter()
        .filter(|history| {
            history
                .metadata
                .as_ref()
                .and_then(|meta| meta.get("message_body"))
                .map_or(false, |s| !s.is_empty())
        })
        .map(|history| MessageRequestEvent {
            friendship_id: history.friendship_id.to_string(),
            acting_user: history.acting_user.to_string(),
            timestamp: history.timestamp.timestamp(),
            body: history
                .metadata
                .clone()
                .unwrap()
                .get("message_body")
                .unwrap()
                .to_string(),
        })
        .collect()
}
