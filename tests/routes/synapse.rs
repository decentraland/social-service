#[cfg(test)]
mod synapse_sniff {

    mod login {
        use std::collections::HashMap;

        use crate::helpers::server::{create_test_db, get_configuration};
        use actix_web::{test, web::Data};
        use faux::when;
        use social_service::{
            components::{
                app::{AppComponents, CustomComponents},
                synapse::{
                    AuthChain, LoginIdentifier, SynapseComponent, SynapseLoginRequest,
                    SynapseLoginResponse,
                },
            },
            get_app_router,
            routes::v1::error::CommonError,
        };

        const URL: &str = "/_matrix/client/r0/login";

        #[actix_web::test]
        async fn should_be_200_and_has_user_in_cache() {
            let config = get_configuration();

            // Just mocks synapse
            let mut mocked_synapse = SynapseComponent::faux();

            when!(mocked_synapse.login).then(|_| {
                let mut mocked_hash_map = HashMap::new();
                let mut mocked_hash_map_2 = HashMap::new();
                mocked_hash_map_2.insert(
                    "base_url".to_string(),
                    "https://synapse.decentraland.zone".to_string(),
                );
                mocked_hash_map.insert("m.homeserver".to_string(), mocked_hash_map_2);
                Ok(SynapseLoginResponse {
                    user_id: "0xA1".to_string(),
                    access_token: "0xA1_TOKEN".to_string(),
                    device_id: "0xA1_DEVICE".to_string(),
                    home_server: "decentraland.zone".to_string(),
                    well_known: mocked_hash_map,
                })
            });

            let mocked_comps = CustomComponents {
                synapse: Some(mocked_synapse),
                db: None,
                users_cache: None,
                redis: None,
            };

            // Manual Setup
            create_test_db(&config.db).await;
            let app_components = AppComponents::new(Some(config), Some(mocked_comps)).await;
            let app_data = Data::new(app_components);

            let router = get_app_router(&app_data);

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

            // Test if all happenned correctly
            let mut users_cache = app_data.users_cache.lock().await;
            let user = users_cache.get_user("0xA1_TOKEN").await;
            assert!(user.is_ok());
            let user = user.unwrap();
            assert_eq!(user, "0xA1");
            assert!(response.status().is_success())
        }

        #[actix_web::test]
        async fn should_be_500_and_not_user_in_cache() {
            let config = get_configuration();

            // Just mocks synapse
            let mut mocked_synapse = SynapseComponent::faux();

            when!(mocked_synapse.login).then(|_| Err(CommonError::Unknown));

            let mocked_comps = CustomComponents {
                synapse: Some(mocked_synapse),
                db: None,
                users_cache: None,
                redis: None,
            };

            // Manual Setup
            create_test_db(&config.db).await;
            let app_components = AppComponents::new(Some(config), Some(mocked_comps)).await;
            let app_data = Data::new(app_components);

            let router = get_app_router(&app_data);

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
    }
}
