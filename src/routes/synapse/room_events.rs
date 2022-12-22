use actix_web::{
    put,
    web::{self, Data},
    HttpMessage, HttpRequest, HttpResponse,
};

use crate::{components::app::AppComponents, middlewares::check_auth::UserId};

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

// async fn process_room_event(user_id:&str, room_id:&str, )
