use std::time::{SystemTime, UNIX_EPOCH};

use dcl_rpc::{
    rpc_protocol::RemoteErrorResponse,
    {service_module_definition::ProcedureContext, stream_protocol::Generator},
};
use futures_util::StreamExt;

use crate::{
    components::{notifications::ChannelPublisher, users_cache::UserId},
    domain::{
        address::Address,
        error::{as_ws_service, CommonError, WsServiceError},
    },
    entities::friendships::{Friendship, FriendshipRepositoryImplementation},
    friendships::{
        request_events_response, subscribe_friendship_events_updates_response,
        update_friendship_response, users_response, FriendshipsServiceServer, InternalServerError,
        Payload, RequestEventsResponse, ServerStreamResponse,
        SubscribeFriendshipEventsUpdatesResponse, UnauthorizedError, UpdateFriendshipPayload,
        UpdateFriendshipResponse, User, Users, UsersResponse,
    },
    synapse::synapse_handler::get_user_id_from_request,
    ws::{
        app::{SocialContext, SocialTransportContext},
        metrics::record_error_response_code,
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
};

#[derive(Debug)]
pub struct MyFriendshipsService {}

pub struct RPCFriendshipsServiceError {
    pub code: u32,
    pub message: String,
}

impl RemoteErrorResponse for RPCFriendshipsServiceError {
    fn error_code(&self) -> u32 {
        self.code
    }

    fn error_message(&self) -> String {
        self.message.to_string()
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
        let request_user_id = get_user_id_from_request(
            &request,
            context.server_context.synapse.clone(),
            context.server_context.users_cache.clone(),
        )
        .await;

        let (friendships_generator, friendships_yielder) = Generator::create();

        let Some(repos) = context.server_context.db.db_repos.clone() else {
            log::error!("Get friends > Db repositories > `repos` is None.");
            record_error_response_code("INTERNAL_SERVER_ERROR");
            tokio::spawn(async move {
                let result = friendships_yielder
                .r#yield(UsersResponse::from_response(users_response::Response::InternalServerError(
                    InternalServerError{ message: "An error occurred while getting the friendships".to_owned() })))
                .await;
                if let Err(err) = result {
                    log::error!("There was an error yielding the error to the friendships generator: {:?}", err);
                };
            });
            return Ok(friendships_generator);
        };

        match request_user_id {
            Err(err) => {
                record_error_response_code("UNAUTHORIZED");
                tokio::spawn(async move {
                    let result = friendships_yielder.r#yield(to_user_response(err)).await;
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
                        record_error_response_code("INTERNAL_SERVER_ERROR");
                        tokio::spawn(async move {
                            let result = friendships_yielder
                                .r#yield(UsersResponse::from_response(users_response::Response::InternalServerError(
                                    InternalServerError{ message: "An error occurred while sending the response to the stream".to_owned() })))
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
        let request_user_id = get_user_id_from_request(
            &request,
            context.server_context.synapse.clone(),
            context.server_context.users_cache.clone(),
        )
        .await;

        match request_user_id {
            Err(err) => {
                record_error_response_code("UNAUTHORIZED");

                return Ok(to_request_events_response(err));
            }
            Ok(user_id) => {
                let social_id = user_id.social_id.clone();
                log::info!("Getting requests events for user: {}", social_id);

                let Some(repos) = context.server_context.db.db_repos.clone() else {
                    log::error!("Get request events > Db repositories > `repos` is None.");
                    record_error_response_code("INTERNAL_SERVER_ERROR");

                    return Ok(RequestEventsResponse::from_response(
                        request_events_response::Response::InternalServerError(InternalServerError { message: "".to_owned() })));
                };

                let requests = repos
                    .friendship_history
                    .get_user_pending_request_events(&user_id.social_id)
                    .await;

                match requests {
                    Err(err) => {
                        log::error!(
                            "Get request events > Get user pending request events > Error: {err}."
                        );
                        record_error_response_code("INTERNAL_SERVER_ERROR");

                        Ok(RequestEventsResponse::from_response(
                            request_events_response::Response::InternalServerError(
                                InternalServerError {
                                    message: "".to_owned(),
                                },
                            ),
                        ))
                    }
                    Ok(requests) => {
                        log::info!("Returning requests events for user {}", social_id);
                        Ok(friendship_requests_as_request_events_response(
                            requests,
                            user_id.social_id,
                        ))
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
        let Some(auth_token) = request.clone().auth_token.take() else {
            record_error_response_code("UNAUTHORIZED");

            return Ok(UpdateFriendshipResponse::from_response(
                update_friendship_response::Response::UnauthorizedError(
                    UnauthorizedError{ message: "`auth_token` was not provided".to_owned() }
                    )
                )
            );
        };

        let request_user_id = get_user_id_from_request(
            &auth_token,
            context.server_context.synapse.clone(),
            context.server_context.users_cache.clone(),
        )
        .await;

        match request_user_id {
            Err(err) => {
                record_error_response_code("UNAUTHORIZED");

                return Ok(to_update_friendship_response(err));
            }
            Ok(user_id) => {
                let friendship_update_response = handle_friendship_update(
                    request.clone(),
                    context.server_context.clone(),
                    user_id.clone().social_id,
                )
                .await;

                match friendship_update_response {
                    Err(err) => {
                        record_error_response_code("INTERNAL"); // TODO: THIS IS HARDCODED!!! IT SHOULD BE READ FROM err

                        return Ok(to_update_friendship_response2(err));
                    }
                    Ok(friendship_update_response) => {
                        let update_response = event_response_as_update_response(
                            request.clone(),
                            friendship_update_response,
                        );

                        match update_response {
                            Err(err) => {
                                record_error_response_code("INTERNAL"); // TODO: THIS IS HARDCODED!!! IT SHOULD BE READ FROM err

                                return Ok(to_update_friendship_response(err));
                            }
                            Ok(update_response) => {
                                // TODO: Use created_at from entity instead of calculating it again (#ISSUE: https://github.com/decentraland/social-service/issues/197)
                                let created_at = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs()
                                    as i64;

                                let publisher = context.server_context.redis_publisher.clone();
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
        let request_user_id = get_user_id_from_request(
            &request,
            context.server_context.synapse.clone(),
            context.server_context.users_cache.clone(),
        )
        .await;

        let (friendships_generator, friendships_yielder) = Generator::create();

        match request_user_id {
            Err(err) => {
                record_error_response_code("INTERNAL"); // TODO: THIS IS HARDCODED!!! IT SHOULD BE READ FROM err

                tokio::spawn(async move {
                    friendships_yielder
                        .r#yield(to_subscribe_friendship_events_updates_response(err))
                        .await
                        .unwrap();
                });
            }
            Ok(user_id) => {
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
                    .insert(Address(user_id.social_id), friendships_yielder.clone());
            }
        }
        Ok(friendships_generator)
    }
}

fn to_user_response(err: CommonError) -> UsersResponse {
    let err = as_ws_service(err);
    match err {
        WsServiceError::Unauthorized(err) => {
            UsersResponse::from_response(users_response::Response::UnauthorizedError(err))
        }
        WsServiceError::InternalServer(err) => {
            UsersResponse::from_response(users_response::Response::InternalServerError(err))
        }
        WsServiceError::BadRequest(err) => {
            UsersResponse::from_response(users_response::Response::InternalServerError(
                InternalServerError {
                    message: err.message,
                },
            )) // TODO: Check if necessary
        }
        WsServiceError::Forbidden(err) => {
            UsersResponse::from_response(users_response::Response::InternalServerError(
                InternalServerError {
                    message: err.message,
                },
            )) // TODO: Check if necessary
        }
        WsServiceError::TooManyRequests(err) => {
            UsersResponse::from_response(users_response::Response::InternalServerError(
                InternalServerError {
                    message: err.message,
                },
            )) // TODO: Check if necessary
        }
    }
}

fn to_request_events_response(err: CommonError) -> RequestEventsResponse {
    let err = as_ws_service(err);
    match err {
        WsServiceError::Unauthorized(err) => RequestEventsResponse::from_response(
            request_events_response::Response::UnauthorizedError(err),
        ),
        WsServiceError::InternalServer(err) => RequestEventsResponse::from_response(
            request_events_response::Response::InternalServerError(err),
        ),
        WsServiceError::BadRequest(err) => {
            RequestEventsResponse::from_response(
                request_events_response::Response::InternalServerError(InternalServerError {
                    message: err.message,
                }),
            ) // TODO: Check if necessary
        }
        WsServiceError::Forbidden(err) => {
            RequestEventsResponse::from_response(
                request_events_response::Response::InternalServerError(InternalServerError {
                    message: err.message,
                }),
            ) // TODO: Check if necessary
        }
        WsServiceError::TooManyRequests(err) => {
            RequestEventsResponse::from_response(
                request_events_response::Response::InternalServerError(InternalServerError {
                    message: err.message,
                }),
            ) // TODO: Check if necessary
        }
    }
}

fn to_update_friendship_response(err: CommonError) -> UpdateFriendshipResponse {
    let err = as_ws_service(err);
    match err {
        WsServiceError::Unauthorized(err) => UpdateFriendshipResponse::from_response(
            update_friendship_response::Response::UnauthorizedError(err),
        ),
        WsServiceError::InternalServer(err) => UpdateFriendshipResponse::from_response(
            update_friendship_response::Response::InternalServerError(err),
        ),
        WsServiceError::BadRequest(err) => UpdateFriendshipResponse::from_response(
            update_friendship_response::Response::BadRequestError(err),
        ),
        WsServiceError::Forbidden(err) => UpdateFriendshipResponse::from_response(
            update_friendship_response::Response::ForbiddenError(err),
        ),
        WsServiceError::TooManyRequests(err) => UpdateFriendshipResponse::from_response(
            update_friendship_response::Response::TooManyRequestsError(err),
        ),
    }
}

fn to_subscribe_friendship_events_updates_response(
    err: CommonError,
) -> SubscribeFriendshipEventsUpdatesResponse {
    let err = as_ws_service(err);
    match err {
        WsServiceError::Unauthorized(err) => {
            SubscribeFriendshipEventsUpdatesResponse::from_response(
                subscribe_friendship_events_updates_response::Response::UnauthorizedError(err),
            )
        }
        WsServiceError::InternalServer(err) => {
            SubscribeFriendshipEventsUpdatesResponse::from_response(
                subscribe_friendship_events_updates_response::Response::InternalServerError(err),
            )
        }
        WsServiceError::BadRequest(err) => SubscribeFriendshipEventsUpdatesResponse::from_response(
            subscribe_friendship_events_updates_response::Response::InternalServerError(
                InternalServerError {
                    message: err.message,
                },
            ),
        ),
        WsServiceError::Forbidden(err) => SubscribeFriendshipEventsUpdatesResponse::from_response(
            subscribe_friendship_events_updates_response::Response::InternalServerError(
                InternalServerError {
                    message: err.message,
                },
            ),
        ),
        WsServiceError::TooManyRequests(err) => {
            SubscribeFriendshipEventsUpdatesResponse::from_response(
                subscribe_friendship_events_updates_response::Response::InternalServerError(
                    InternalServerError {
                        message: err.message,
                    },
                ),
            )
        }
    }
}

// Delete this method
fn to_update_friendship_response2(err: WsServiceError) -> UpdateFriendshipResponse {
    match err {
        WsServiceError::Unauthorized(err) => UpdateFriendshipResponse::from_response(
            update_friendship_response::Response::UnauthorizedError(err),
        ),
        WsServiceError::InternalServer(err) => UpdateFriendshipResponse::from_response(
            update_friendship_response::Response::InternalServerError(err),
        ),
        WsServiceError::BadRequest(err) => UpdateFriendshipResponse::from_response(
            update_friendship_response::Response::BadRequestError(err),
        ),
        WsServiceError::Forbidden(err) => UpdateFriendshipResponse::from_response(
            update_friendship_response::Response::ForbiddenError(err),
        ),
        WsServiceError::TooManyRequests(err) => UpdateFriendshipResponse::from_response(
            update_friendship_response::Response::TooManyRequestsError(err),
        ),
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
