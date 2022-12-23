use actix_web::{
    put,
    web::{self, Data},
    HttpMessage, HttpRequest, HttpResponse,
};
use serde::{de::Visitor, Deserialize, Serialize};

use crate::{
    components::{app::AppComponents, database::DatabaseComponent, synapse::SynapseComponent},
    middlewares::check_auth::{Token, UserId},
    routes::v1::error::CommonError,
};

#[derive(Deserialize, Serialize)]
pub struct RoomEventResponse {
    event_id: String,
}

#[derive(Deserialize, Serialize)]
pub struct RoomEventRequestBody {
    pub r#type: FriendshipEvent,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
// #[serde(rename_all = "lowercase")]
pub enum FriendshipEvent {
    // #[serde(rename = "request")]
    REQUEST, // Send a friendship request
    #[serde(rename = "cancel")]
    CANCEL, // Cancel a friendship request
    ACCEPT,  // Accept a friendship request
    REJECT,  // Reject a friendship request
    DELETE,  // Delete an existing friendship
}

impl FriendshipEvent {
    fn as_str(&self) -> String {
        format!("{self:?}").to_lowercase()
    }
}

#[put("/_matrix/client/r0/rooms/{room_id}/state/org.decentraland.friendship")]
pub async fn room_event_handler(
    req: HttpRequest,
    body: web::Json<RoomEventRequestBody>,
    room_id: web::Path<String>,
    app_data: Data<AppComponents>,
) -> Result<HttpResponse, CommonError> {
    let extensions = req.extensions();
    let logged_in_user = extensions.get::<UserId>().unwrap().0.as_str();
    let token = extensions.get::<Token>().unwrap().0.as_str();
    // let

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
    user_id: &str,
    token: &str,
    room_id: &str,
    room_event: FriendshipEvent,
    _db: &DatabaseComponent,
    synapse: &SynapseComponent,
) -> Result<RoomEventResponse, CommonError> {
    synapse.store_room_event(token, room_id, room_event).await
}
