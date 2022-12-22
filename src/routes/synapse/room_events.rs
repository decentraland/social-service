use actix_web::{
    put,
    web::{self, Data},
    HttpMessage, HttpRequest, HttpResponse,
};

use crate::{
    components::{app::AppComponents, database::DatabaseComponent, synapse::SynapseComponent},
    middlewares::check_auth::UserId,
};

#[put("/_matrix/client/r0/rooms/{room_id}/state/org.decentraland.friendship")]
pub async fn room_event_handler(
    req: HttpRequest,
    _room_id: web::Path<String>,
    _app_data: Data<AppComponents>,
) -> HttpResponse {
    let extensions = req.extensions();
    let _logged_in_user = extensions.get::<UserId>().unwrap().0.as_str();

    HttpResponse::Ok().finish()
}

async fn process_room_event(
    user_id: &str,
    room_id: &str,
    db: &DatabaseComponent,
    synapse: &SynapseComponent,
) {
}

#[derive(Debug)]
enum FriendshipEvent {
    REQUEST, // Send a friendship request
    CANCEL,  // Cancel a friendship request
    ACCEPT,  // Accept a friendship request
    REJECT,  // Reject a friendship request
    DELETE,  // Delete an existing friendship
}

impl FriendshipEvent {
    fn as_str(&self) -> String {
        format!("{self:?}").to_lowercase()
    }
}
