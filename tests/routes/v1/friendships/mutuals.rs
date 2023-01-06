use std::collections::HashMap;

use actix_http::StatusCode;
use actix_web::{test, web::Data};
use social_service::{
    components::{app::AppComponents, database::DatabaseComponent},
    get_app_router,
    routes::v1::friendships::types::{FriendshipFriend, FriendshipsResponse},
};

use super::utils::add_friendship;
use crate::common::*;

// Get friends should return list of friends
#[actix_web::test]
async fn test_get_mutual_friends() {
    let user_id_a = "user-a";
    let user_id_b = "user-b";
    let user_id_c = "user-c";
    let user_id_d = "user-d";
    let user_id_e = "user-e";
    let user_id_f = "user-f";

    let token = "token-user-a";

    let mut token_to_user_id: HashMap<String, String> = HashMap::new();
    token_to_user_id.insert(token.to_string(), user_id_a.to_string());

    let mock_server = who_am_i_synapse_mock_server(token_to_user_id).await;
    let mut config = get_configuration().await;
    config.synapse.url = mock_server.uri();

    let app_components = AppComponents::new(Some(config)).await;
    let app_data = Data::new(app_components);

    let router = get_app_router(&app_data);

    let app = test::init_service(router).await;

    // user relations:
    // a -> c -> b (in db: a -> c, b -> c) checks when query for (address_1, address_1)
    add_friendship(&app_data.db, (user_id_a, user_id_c)).await;
    add_friendship(&app_data.db, (user_id_b, user_id_c)).await;

    // a -> d -> b (in db: a -> d, d -> b) checks when query for (address_1, address_2)
    add_friendship(&app_data.db, (user_id_a, user_id_d)).await;
    add_friendship(&app_data.db, (user_id_d, user_id_b)).await;

    // a -> e -> b (in db: e -> a, b -> e) checks when query for (address_2, address_1)
    add_friendship(&app_data.db, (user_id_e, user_id_a)).await;
    add_friendship(&app_data.db, (user_id_b, user_id_e)).await;
    // a -> f -> b (in db: f -> a, f -> b) checks when query for (address_2, address_2)
    add_friendship(&app_data.db, (user_id_f, user_id_a)).await;
    add_friendship(&app_data.db, (user_id_f, user_id_b)).await;

    let url = format!("/v1/friendships/{user_id_a}");

    let header = ("authorization", format!("Bearer {}", token));
    let req = test::TestRequest::get()
        .uri(url.as_str())
        .append_header(header)
        .to_request();

    let response = test::call_service(&app, req).await;

    assert_eq!(response.status(), StatusCode::OK);

    let friendships_response: FriendshipsResponse = test::read_body_json(response).await;
    let friend_address = &friendships_response.friendships;

    assert!(friend_address.contains(&FriendshipFriend {
        address: user_id_c.to_string()
    }));
    assert!(friend_address.contains(&FriendshipFriend {
        address: user_id_d.to_string()
    }));
    assert!(friend_address.contains(&FriendshipFriend {
        address: user_id_e.to_string()
    }));
    assert!(friend_address.contains(&FriendshipFriend {
        address: user_id_f.to_string()
    }));
}
