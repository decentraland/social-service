mod common;
pub use common::*;
use dcl_http_prom_metrics::HttpMetricsCollectorBuilder;

use std::collections::HashMap;
use wiremock::{
    matchers::{method, path},
    Mock, ResponseTemplate,
};

use actix_web::{test, web::Data};
use social_service::{
    api::app::get_app_router,
    components::{
        app::AppComponents,
        synapse::{AuthChain, LoginIdentifier, SynapseLoginRequest, SynapseLoginResponse},
        users_cache::UserId,
    },
};

const URL: &str = "/_matrix/client/r0/login";

#[actix_web::test]
async fn should_be_200_and_has_user_in_cache() {
    let mut mocked_hash_map = HashMap::new();
    let mut mocked_hash_map_2 = HashMap::new();
    mocked_hash_map_2.insert(
        "base_url".to_string(),
        "https://synapse.decentraland.zone".to_string(),
    );
    mocked_hash_map.insert("m.homeserver".to_string(), mocked_hash_map_2);
    let login_response = SynapseLoginResponse {
        user_id: "0xA1".to_string(),
        social_user_id: None,
        access_token: "0xA1_TOKEN".to_string(),
        device_id: "0xA1_DEVICE".to_string(),
        home_server: "decentraland.zone".to_string(),
        well_known: mocked_hash_map,
    };
    let synapse_server = create_synapse_mock_server().await;

    Mock::given(method("POST"))
        .and(path(URL))
        .respond_with(ResponseTemplate::new(200).set_body_json(login_response))
        .mount(&synapse_server)
        .await;

    let mut config = get_configuration().await;
    config.synapse.url = synapse_server.uri();

    // Manual Setup
    let app_components = AppComponents::new(Some(config)).await;
    let app_data = Data::new(app_components);

    let http_metrics_collector = Data::new(HttpMetricsCollectorBuilder::default().build());

    let router = get_app_router(&app_data, &http_metrics_collector);

    let app = test::init_service(router).await;

    let login_req = SynapseLoginRequest {
        _type: "m.login.decentraland".to_string(),
        identifier: LoginIdentifier {
            _type: "m.id.user".to_string(),
            user: "0xA1".to_string(),
        },
        timestamp: "1671211160629".to_string(),
        auth_chain: vec![
            AuthChain {
                _type: "SIGNER".to_string(),
                payload: "0xA1".to_string(),
                signature: "".to_string(),
            },
            AuthChain {
                _type: "ECDSA_EPHEMERAL".to_string(),
                payload: "stuff".to_string(),
                signature: "stuff".to_string(),
            },
            AuthChain {
                _type: "ECDSA_SIGNED_ENTITY".to_string(),
                payload: "stuff".to_string(),
                signature: "stuff".to_string(),
            },
        ],
    };

    let req = test::TestRequest::post()
        .uri(URL)
        .set_json(login_req)
        .to_request();

    let response = test::call_service(&app, req).await;

    assert!(response.status().is_success());

    // Test if all happenned correctly
    let mut users_cache = app_data.users_cache.lock().await;
    let user = users_cache.get_user("0xA1_TOKEN").await;
    assert!(user.is_ok());
    let user = user.unwrap();
    assert_eq!(
        user,
        UserId {
            social_id: "0xA1".to_string(),
            synapse_id: "0xA1".to_string()
        }
    );
}

#[actix_web::test]
async fn should_be_500_and_not_user_in_cache() {
    let synapse_server = create_synapse_mock_server().await;

    Mock::given(method("GET"))
        .and(path(URL))
        .respond_with(ResponseTemplate::new(500))
        .mount(&synapse_server)
        .await;

    let mut config = get_configuration().await;
    config.synapse.url = synapse_server.uri();

    // Manual Setup
    let app_components = AppComponents::new(Some(config)).await;
    let app_data = Data::new(app_components);

    let http_metrics_collector = Data::new(HttpMetricsCollectorBuilder::default().build());

    let router = get_app_router(&app_data, &http_metrics_collector);

    let app = test::init_service(router).await;

    let login_req = SynapseLoginRequest {
        _type: "m.login.decentraland".to_string(),
        identifier: LoginIdentifier {
            _type: "m.id.user".to_string(),
            user: "0xB1".to_string(),
        },
        timestamp: "1671211160629".to_string(),
        auth_chain: vec![
            AuthChain {
                _type: "SIGNER".to_string(),
                payload: "0xB1".to_string(),
                signature: "".to_string(),
            },
            AuthChain {
                _type: "ECDSA_EPHEMERAL".to_string(),
                payload: "stuff".to_string(),
                signature: "stuff".to_string(),
            },
            AuthChain {
                _type: "ECDSA_SIGNED_ENTITY".to_string(),
                payload: "stuff".to_string(),
                signature: "stuff".to_string(),
            },
        ],
    };

    let req = test::TestRequest::post()
        .uri(URL)
        .set_json(login_req)
        .to_request();

    let response = test::call_service(&app, req).await;

    // Test if all happenned correctly
    let mut users_cache = app_data.users_cache.lock().await;
    let user = users_cache.get_user("0xB1_TOKEN").await;
    assert!(user.is_err());
    assert!(response.status().is_server_error())
}
