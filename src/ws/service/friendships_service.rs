use std::time::{SystemTime, UNIX_EPOCH};

use dcl_rpc::{
    rpc_protocol::RemoteErrorResponse,
    {service_module_definition::ProcedureContext, stream_protocol::Generator},
};
use futures_util::StreamExt;

use crate::{
    components::{notifications::ChannelPublisher, users_cache::UserId},
    entities::friendships::{Friendship, FriendshipRepositoryImplementation},
    friendships::{
        request_events_response, subscribe_friendship_events_updates_response,
        update_friendship_response, users_response, FriendshipsServiceServer, Payload,
        RequestEventsResponse, ServerStreamResponse, SubscribeFriendshipEventsUpdatesResponse,
        UpdateFriendshipPayload, UpdateFriendshipResponse, User, Users, UsersResponse,
    },
    models::address::Address,
    ws::{
        app::{record_error_response_code, SocialContext, SocialTransportContext},
        service::errors::{as_service_error, DomainErrorCode},
    },
};

use super::{
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

pub enum RPCFriendshipsServiceError {}

impl RemoteErrorResponse for RPCFriendshipsServiceError {
    fn error_code(&self) -> u32 {
        todo!()
    }

    fn error_message(&self) -> String {
        todo!()
    }
}

#[async_trait::async_trait]
impl FriendshipsServiceServer<SocialContext, RPCFriendshipsServiceError> for MyFriendshipsService {
    #[tracing::instrument(name = "RPC SERVER > Get Friends Generator", skip(request, context))]
    async fn get_friends(
        &self,
        request: Payload,
        context: ProcedureContext<SocialContext>,
    ) -> Result<ServerStreamResponse<UsersResponse>, RPCFriendshipsServiceError> {
        // Get user id with the given Authentication Token.
        let request_user_id = get_user_id_from_request(
            &request,
            context.server_context.synapse.clone(),
            context.server_context.users_cache.clone(),
        )
        .await;

        let (friendships_generator, friendships_yielder) = Generator::create();

        let Some(repos) = context.server_context.db.db_repos.clone() else {
            log::error!("Get friends > Db repositories > `repos` is None.");
            record_error_response_code(DomainErrorCode::InternalServerError as u32);
            tokio::spawn(async move {
                let result = friendships_yielder
                .r#yield(UsersResponse::from_response(users_response::Response::Error(
                        as_service_error(
                            DomainErrorCode::InternalServerError,
                            "An error occurred while getting the friendships",
                        )
                )))
                .await;
                if let Err(err) = result {
                    log::error!("There was an error yielding the error to the friendships generator: {:?}", err);
                };
            });
            return Ok(friendships_generator);
        };

        match request_user_id {
            Err(err) => {
                record_error_response_code(err.code as u32);
                tokio::spawn(async move {
                    let result = friendships_yielder
                        .r#yield(UsersResponse::from_response(
                            users_response::Response::Error(err),
                        ))
                        .await;
                    if let Err(err) = result {
                        log::error!("There was an error yielding the error to the friendships generator: {:?}", err);
                    };
                });
            }
            Ok(user_id) => {
                let social_id = user_id.social_id.clone();
                log::info!("Getting all friends for user: {}", social_id);

                let Ok(mut friendship) = repos
                    .friendships
                    .get_user_friends_stream(&user_id.social_id, true)
                    .await else {
                        log::error!(
                            "Get friends > Get user friends stream > Error: There was an error accessing to the friendships repository."
                        );
                        record_error_response_code(
                            DomainErrorCode::InternalServerError as u32,
                        );
                        tokio::spawn(async move {
                            let error = as_service_error(DomainErrorCode::InternalServerError, "An error occurred while sending the response to the stream");
                            let result = friendships_yielder
                                .r#yield(
                                    UsersResponse::from_response(users_response::Response::Error(error)))
                                .await;
                            if let Err(err) = result {
                                log::error!("There was an error yielding the error to the friendships generator: {:?}", err);
                            };
                        });
                        return Ok(friendships_generator);
                    };

                tokio::spawn(async move {
                    let mut users = Users::default();

                    while let Some(friendship) = friendship.next().await {
                        users.users.push(build_user(friendship, user_id.clone()));

                        if users.users.len() == 5 {
                            // TODO: Move this value (5) to a Env Variable, Config or sth like that (#ISSUE: https://github.com/decentraland/social-service/issues/199)
                            let result = friendships_yielder
                                .r#yield(UsersResponse::from_response(
                                    users_response::Response::Users(users.clone()),
                                ))
                                .await;
                            if let Err(err) = result {
                                log::error!("There was an error yielding the response to the friendships generator: {:?}", err);
                                // TODO: If there was an error yielding the correct response, does it make sense to try to yield the error one?
                                break;
                            };
                            users = Users::default();
                        }
                    }
                    let result = friendships_yielder
                        .r#yield(UsersResponse::from_response(
                            users_response::Response::Users(users),
                        ))
                        .await;
                    if let Err(err) = result {
                        log::error!("There was an error yielding the response to the friendships generator: {:?}", err);
                        // TODO: If there was an error yielding the correct response, does it make sense to try to yield the error one?
                    };
                });
                log::info!("Returning generator for all friends for user {}", social_id);
            }
        }
        Ok(friendships_generator)
    }

    #[tracing::instrument(name = "RPC SERVER > Get Request Events", skip(request, context))]
    async fn get_request_events(
        &self,
        request: Payload,
        context: ProcedureContext<SocialContext>,
    ) -> Result<RequestEventsResponse, RPCFriendshipsServiceError> {
        // Get user id with the given Authentication Token.
        let request_user_id = get_user_id_from_request(
            &request,
            context.server_context.synapse.clone(),
            context.server_context.users_cache.clone(),
        )
        .await;

        match request_user_id {
            Err(err) => {
                // Register failure in metrics
                record_error_response_code(err.code as u32);

                return Ok(RequestEventsResponse {
                    response: Some(request_events_response::Response::Error(err)),
                });
            }
            Ok(user_id) => {
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
                                    DomainErrorCode::InternalServerError as u32,
                                );

                                Ok(RequestEventsResponse {
                                    response: Some(request_events_response::Response::Error(
                                        as_service_error(DomainErrorCode::InternalServerError, ""),
                                    )),
                                })
                            }
                        }
                    }
                    None => {
                        log::error!("Get request events > Db repositories > `repos` is None.");
                        record_error_response_code(DomainErrorCode::InternalServerError as u32);

                        Ok(RequestEventsResponse {
                            response: Some(request_events_response::Response::Error(
                                as_service_error(DomainErrorCode::InternalServerError, ""),
                            )),
                        })
                    }
                }
            }
        }
    }

    #[tracing::instrument(name = "RPC SERVER > Update Friendship Event", skip(request, context))]
    async fn update_friendship_event(
        &self,
        request: UpdateFriendshipPayload,
        context: ProcedureContext<SocialContext>,
    ) -> Result<UpdateFriendshipResponse, RPCFriendshipsServiceError> {
        // Get user id with the given Authentication Token.
        let auth_token = request.clone().auth_token.take();
        match auth_token {
            None => {
                // Register failure in metrics
                record_error_response_code(DomainErrorCode::Unauthorized as u32);

                return Ok(UpdateFriendshipResponse {
                    response: Some(update_friendship_response::Response::Error(
                        as_service_error(
                            DomainErrorCode::Unauthorized,
                            "`auth_token` was not provided",
                        ),
                    )),
                });
            }
            Some(auth_token) => {
                // Get user id with the given Authentication Token.
                let request_user_id = get_user_id_from_request(
                    &auth_token,
                    context.server_context.synapse.clone(),
                    context.server_context.users_cache.clone(),
                )
                .await;

                match request_user_id {
                    Err(err) => {
                        // Register failure in metrics
                        record_error_response_code(err.code as u32);

                        return Ok(UpdateFriendshipResponse {
                            response: Some(update_friendship_response::Response::Error(err)),
                        });
                    }
                    Ok(user_id) => {
                        // Handle friendship event update
                        let friendship_update_response = handle_friendship_update(
                            request.clone(),
                            context.server_context.clone(),
                            user_id.clone().social_id,
                        )
                        .await;

                        match friendship_update_response {
                            Err(err) => {
                                // Register failure in metrics
                                record_error_response_code(err.code as u32);

                                return Ok(UpdateFriendshipResponse {
                                    response: Some(update_friendship_response::Response::Error(
                                        err,
                                    )),
                                });
                            }
                            Ok(friendship_update_response) => {
                                // Convert event response to update response
                                let update_response = event_response_as_update_response(
                                    request.clone(),
                                    friendship_update_response,
                                );

                                match update_response {
                                    Err(err) => {
                                        // Register failure in metrics
                                        record_error_response_code(err.code as u32);

                                        return Ok(UpdateFriendshipResponse {
                                            response: Some(
                                                update_friendship_response::Response::Error(err),
                                            ),
                                        });
                                    }
                                    Ok(update_response) => {
                                        // TODO: Use created_at from entity instead of calculating it again (#ISSUE: https://github.com/decentraland/social-service/issues/197)
                                        let created_at = SystemTime::now()
                                            .duration_since(UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs()
                                            as i64;

                                        let publisher =
                                            context.server_context.redis_publisher.clone();
                                        if let Some(event) = request.clone().event {
                                            tokio::spawn(async move {
                                                if let Some(update_friendship_payload_as_event) =
                                                    update_friendship_payload_as_event(
                                                        event,
                                                        user_id.social_id.as_str(),
                                                        created_at,
                                                    )
                                                {
                                                    publisher
                                                        .publish(update_friendship_payload_as_event)
                                                        .await;
                                                }
                                            });
                                        };
                                        Ok(update_response)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
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
        RPCFriendshipsServiceError,
    > {
        // Get user id with the given Authentication Token.
        let request_user_id = get_user_id_from_request(
            &request,
            context.server_context.synapse.clone(),
            context.server_context.users_cache.clone(),
        )
        .await;

        match request_user_id {
            Err(err) => {
                // Register failure in metrics
                record_error_response_code(err.code as u32);

                let (friendships_generator, friendships_yielder) = Generator::create();
                tokio::spawn(async move {
                    friendships_yielder
                        .r#yield(SubscribeFriendshipEventsUpdatesResponse {
                            response: Some(
                                subscribe_friendship_events_updates_response::Response::Error(err),
                            ),
                        })
                        .await
                        .unwrap();
                });
                return Ok(friendships_generator);
            }
            Ok(user_id) => {
                let (friendship_updates_generator, friendship_updates_yielder) =
                    Generator::create();

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
    }
}

fn build_user(friendship: Friendship, user_id: UserId) -> User {
    let address1: String = friendship.address_1;
    let address2: String = friendship.address_2;
    match address1.eq_ignore_ascii_case(&user_id.social_id) {
        true => User { address: address2 },
        false => User { address: address1 },
    }
}

impl UsersResponse {
    fn from_response(response: users_response::Response) -> Self {
        Self {
            response: Some(response),
        }
    }
}
