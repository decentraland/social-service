use std::time::{SystemTime, UNIX_EPOCH};

use dcl_rpc::{
    rpc_protocol::RemoteErrorResponse,
    {service_module_definition::ProcedureContext, stream_protocol::Generator},
};
use futures_util::StreamExt;

use crate::{
    components::notifications::ChannelPublisher,
    entities::friendships::FriendshipRepositoryImplementation,
    friendships::{
        users_response, FriendshipsServiceServer, Payload, RequestEventsResponse,
        ServerStreamResponse, SubscribeFriendshipEventsUpdatesResponse, UpdateFriendshipPayload,
        UpdateFriendshipResponse, User, Users, UsersResponse,
    },
    models::address::Address,
    ws::app::{record_error_response_code, SocialContext, SocialTransportContext},
};

use super::{
    errors::FriendshipsServiceError,
    friendship_event_updates::handle_friendship_update,
    mapper::{
        events::{
            event_response_as_update_response, friendship_requests_as_request_events_response,
        },
        payload_to_response::update_friendship_payload_as_event,
    },
    synapse_handler::get_user_id_from_request,
};

#[derive(Debug)]
pub struct MyFriendshipsService {}

#[async_trait::async_trait]
impl FriendshipsServiceServer<SocialContext, FriendshipsServiceError> for MyFriendshipsService {
    #[tracing::instrument(name = "RPC SERVER > Get Friends Generator", skip(request, context))]
    async fn get_friends(
        &self,
        request: Payload,
        context: ProcedureContext<SocialContext>,
    ) -> Result<ServerStreamResponse<UsersResponse>, FriendshipsServiceError> {
        // Get user id with the given Authentication Token.
        let request_user_id = get_user_id_from_request(
            &request,
            context.server_context.synapse.clone(),
            context.server_context.users_cache.clone(),
        )
        .await;

        match request_user_id {
            Err(err) => {
                let error_code = err.error_code();
                // Register failure in metrics
                record_error_response_code(error_code);

                let (generator, generator_yielder) = Generator::create();
                tokio::spawn(async move {
                    generator_yielder
                        .r#yield(UsersResponse {
                            response: Some(users_response::Response::Error(error_code as i32)),
                        })
                        .await
                        .unwrap();
                });
                return Ok(generator);
            }
            Ok(user_id) => {
                let social_id = user_id.social_id.clone();
                log::info!("Getting all friends for user: {}", social_id);
                // Look for users friends
                let mut friendship = match context.server_context.db.db_repos.clone() {
                    Some(repos) => {
                        let friendship = repos
                            .friendships
                            .get_user_friends_stream(&user_id.social_id, true)
                            .await;
                        match friendship {
                            Ok(it) => it,
                            Err(err) => {
                                log::error!(
                                    "Get friends > Get user friends stream > Error: {err}."
                                );
                                record_error_response_code(
                                    FriendshipsServiceError::InternalServerError.error_code(),
                                );
                                return Err(FriendshipsServiceError::InternalServerError);
                            }
                        }
                    }
                    None => {
                        log::error!("Get friends > Db repositories > `repos` is None.");
                        record_error_response_code(
                            FriendshipsServiceError::InternalServerError.error_code(),
                        );
                        return Err(FriendshipsServiceError::InternalServerError);
                    }
                };

                let (generator, generator_yielder) = Generator::create();

                tokio::spawn(async move {
                    let mut users = Users::default();
                    // Map Frienships to Users
                    loop {
                        let friendship = friendship.next().await;
                        match friendship {
                            Some(friendship) => {
                                let user: User = {
                                    let address1: String = friendship.address_1;
                                    let address2: String = friendship.address_2;
                                    match address1.eq_ignore_ascii_case(&user_id.social_id) {
                                        true => User { address: address2 },
                                        false => User { address: address1 },
                                    }
                                };

                                let users_len = users.users.len();

                                users.users.push(user);

                                // TODO: Move this value (5) to a Env Variable, Config or sth like that (#ISSUE: https://github.com/decentraland/social-service/issues/199)
                                if users_len == 5 {
                                    generator_yielder
                                        .r#yield(UsersResponse {
                                            response: Some(users_response::Response::Users(users)),
                                        })
                                        .await
                                        .unwrap();
                                    users = Users::default();
                                }
                            }
                            None => {
                                generator_yielder
                                    .r#yield(UsersResponse {
                                        response: Some(users_response::Response::Users(users)),
                                    })
                                    .await
                                    .unwrap();
                                break;
                            }
                        }
                    }
                });

                log::info!("Returning generator for all friends for user {}", social_id);
                return Ok(generator);
            }
        }
    }

    #[tracing::instrument(name = "RPC SERVER > Get Request Events", skip(request, context))]
    async fn get_request_events(
        &self,
        request: Payload,
        context: ProcedureContext<SocialContext>,
    ) -> Result<RequestEventsResponse, FriendshipsServiceError> {
        // Get user id with the given Authentication Token.
        let user_id = get_user_id_from_request(
            &request,
            context.server_context.synapse.clone(),
            context.server_context.users_cache.clone(),
        )
        .await
        .map_err(|err| {
            record_error_response_code(err.error_code());
            err
        })?;

        let social_id = user_id.social_id.clone();
        log::info!("Getting requests events for user: {}", social_id);
        // Look for users requests
        match context.server_context.db.db_repos.clone() {
            Some(repos) => {
                let requests = repos
                    .friendship_history
                    .get_user_pending_request_events(&user_id.social_id)
                    .await;
                match requests {
                    Ok(requests) => {
                        log::info!("Returning requests events for user {}", social_id);
                        Ok(friendship_requests_as_request_events_response(
                            requests,
                            user_id.social_id,
                        ))
                    }
                    Err(err) => {
                        log::error!(
                            "Get request events > Get user pending request events > Error: {err}."
                        );
                        record_error_response_code(
                            FriendshipsServiceError::InternalServerError.error_code(),
                        );
                        Err(FriendshipsServiceError::InternalServerError)
                    }
                }
            }
            None => {
                log::error!("Get request events > Db repositories > `repos` is None.");
                record_error_response_code(
                    FriendshipsServiceError::InternalServerError.error_code(),
                );
                return Err(FriendshipsServiceError::InternalServerError);
            }
        }
    }

    #[tracing::instrument(name = "RPC SERVER > Update Friendship Event", skip(request, context))]
    async fn update_friendship_event(
        &self,
        request: UpdateFriendshipPayload,
        context: ProcedureContext<SocialContext>,
    ) -> Result<UpdateFriendshipResponse, FriendshipsServiceError> {
        // Get user id with the given Authentication Token.
        let auth_token = request.clone().auth_token.take().ok_or_else(|| {
            FriendshipsServiceError::Unauthorized("`auth_token` was not provided".to_string())
        })?;
        let user_id = get_user_id_from_request(
            &auth_token,
            context.server_context.synapse.clone(),
            context.server_context.users_cache.clone(),
        )
        .await
        .map_err(|err| {
            record_error_response_code(err.error_code());
            err
        })?;

        // Handle friendship event update
        let friendship_update_response = handle_friendship_update(
            request.clone(),
            context.server_context.clone(),
            user_id.clone().social_id,
        )
        .await
        .map_err(|err| {
            record_error_response_code(err.error_code());
            err
        })?;

        // Convert event response to update response
        let update_response =
            event_response_as_update_response(request.clone(), friendship_update_response)
                .map_err(|err| {
                    record_error_response_code(err.error_code());
                    err
                })?;

        // TODO: Use created_at from entity instead of calculating it again (#ISSUE: https://github.com/decentraland/social-service/issues/197)
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let publisher = context.server_context.redis_publisher.clone();
        if let Some(event) = request.clone().event {
            tokio::spawn(async move {
                if let Some(update_friendship_payload_as_event) = update_friendship_payload_as_event(
                    event,
                    user_id.social_id.as_str(),
                    created_at,
                ) {
                    publisher.publish(update_friendship_payload_as_event).await;
                }
            });
        };

        Ok(update_response)
    }

    #[tracing::instrument(
        name = "RPC SERVER > Subscribe to friendship updates",
        skip(request, context)
    )]
    async fn subscribe_friendship_events_updates(
        &self,
        request: Payload,
        context: ProcedureContext<SocialContext>,
    ) -> Result<
        ServerStreamResponse<SubscribeFriendshipEventsUpdatesResponse>,
        FriendshipsServiceError,
    > {
        // Get user id with the given Authentication Token.
        let user_id = get_user_id_from_request(
            &request,
            context.server_context.synapse.clone(),
            context.server_context.users_cache.clone(),
        )
        .await
        .map_err(|err| {
            record_error_response_code(err.error_code());
            err
        })?;

        let (friendship_updates_generator, friendship_updates_yielder) = Generator::create();

        // Attach transport_id to the context by transport
        context
            .server_context
            .transport_context
            .write()
            .await
            .insert(
                context.transport_id,
                SocialTransportContext {
                    address: Address(user_id.social_id.to_string()),
                },
            );

        // Attach generator to the context by user_id
        context
            .server_context
            .friendships_events_generators
            .write()
            .await
            .insert(
                Address(user_id.social_id),
                friendship_updates_yielder.clone(),
            );

        Ok(friendship_updates_generator)
    }
}
