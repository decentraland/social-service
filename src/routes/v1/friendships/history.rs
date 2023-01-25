use actix_http::HttpMessage;
use actix_web::{
    get,
    web::{self, Data},
    HttpRequest, HttpResponse,
};

use crate::{
    components::app::AppComponents, entities::friendship_history::FriendshipHistory,
    middlewares::check_auth::UserId, routes::v1::friendships::errors::FriendshipsError,
};

// 1st QUESTION:
// Ideally, I would like to receive the timestamp_from and timestamp_to in the request to narrow the query.
// So, would it be a good idea to have sth like this?:
// the fn get_sent_messages_request_event() also receives the param:
// body: web::Json<RequestEventRequestBody>,

// Where RequestEventRequestBody is something like this:
// pub struct RequestEventRequestBody {
//   pub timestamp_from: i64,
//   pub timestamp_to: i64
// }

// 2nd QUESTION:
// Should we have a dedicated endpoint?
#[get("/v1/friendships/{friendshipId}/request_message")]
async fn get_sent_messages_request_event(
    req: HttpRequest,
    _friendship_id: web::Path<String>,
    _app_data: Data<AppComponents>,
) -> Result<HttpResponse, FriendshipsError> {
    let _logged_in_user = req
        .extensions()
        .get::<UserId>()
        .expect("to have a UserId")
        .clone();

    todo!();

    // Get the history of friendship request events.

    // match &app_data.db.db_repos {
    //     Some(repos) => {
    //         let (history, _) = repos
    //             .friendship_history
    //             .get_friendship_request_event_history(
    //                 friendship_id,
    //                 body.timestamp_from,
    //                 body.timestamp_to,
    //                 None,
    //             )
    //             .await;
    //         match history {
    //             Err(_) => Err(FriendshipsError::CommonError(CommonError::Unknown)),
    //             Ok(history) => {
    //                 // Get request events with messages
    //                 let response = get_request_events_with_messages(history);
    //                 Ok(HttpResponse::Ok().json(response));
    //             }
    //         }
    //     }
    //     None => Err(FriendshipsError::CommonError(CommonError::NotFound)),
    // }
}

/// Filters the input friendship_history and returns a filtered Vec<FriendshipHistory>
/// It checks if the metadata field of each FriendshipHistory struct contains the key "message_body"
/// and if it contains a non-empty value. If the check returns true, the struct is included in the filtered vector.
fn _get_request_events_with_messages(
    friendship_history: Vec<FriendshipHistory>,
) -> Vec<FriendshipHistory> {
    friendship_history
        .iter()
        .filter(|history| {
            history
                .metadata
                .as_ref()
                .and_then(|meta| meta.get("message_body"))
                .map_or(false, |s| !s.is_empty())
        })
        .map(|history| FriendshipHistory {
            friendship_id: history.friendship_id,
            event: history.event,
            acting_user: history.acting_user.to_string(),
            timestamp: history.timestamp,
            metadata: history.metadata.clone(),
        })
        .collect()
}
