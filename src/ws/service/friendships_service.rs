use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use dcl_rpc::{
    rpc_protocol::RemoteErrorResponse,
    {service_module_definition::ProcedureContext, stream_protocol::Generator},
};
use futures_util::StreamExt;
use tokio::sync::Mutex;

use crate::{
    components::{
        notifications::ChannelPublisher,
        synapse::SynapseComponent,
        users_cache::{get_user_id_from_token, UserId, UsersCacheComponent},
    },
    domain::{address::Address, error::CommonError},
    entities::friendships::{Friendship, FriendshipRepositoryImplementation},
    friendships::{
        request_events_response, update_friendship_response, users_response,
        FriendshipsServiceServer, InternalServerError, Payload, RequestEventsResponse,
        ServerStreamResponse, SubscribeFriendshipEventsUpdatesResponse, UnauthorizedError,
        UpdateFriendshipPayload, UpdateFriendshipResponse, User, Users, UsersResponse,
    },
    ws::{
        app::{SocialContext, SocialTransportContext},
        metrics::{record_error_response_code, Procedure},
    },
};

use super::{
    friendship_event_updates::handle_friendship_update,
    mapper::{
        event::{
            event_response_as_update_response, friendship_requests_as_request_events_response,
            update_friendship_payload_as_event, update_request_as_event_payload,
        },
        payload::get_synapse_token,
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
            let error = InternalServerError{ message: "An error occurred while getting the friendships".to_owned() };
            record_error_response_code(error.clone().into(), Procedure::GetFriends);
            let result = friendships_yielder
            .r#yield(UsersResponse::from_response(users_response::Response::InternalServerError(
                error)))
            .await;
            if let Err(err) = result {
                log::error!("There was an error yielding the error to the friendships generator: {:?}", err);
            };
            return Ok(friendships_generator);
        };

        match request_user_id {
            Err(err) => {
                record_error_response_code(err.clone().into(), Procedure::GetFriends);
                let result = friendships_yielder.r#yield(err.into()).await;
                if let Err(err) = result {
                    log::error!(
                        "There was an error yielding the error to the friendships generator: {:?}",
                        err
                    );
                };
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
                        let error = InternalServerError{ message: "An error occurred while sending the response to the stream".to_owned() };
                        record_error_response_code(error.clone().into(), Procedure::GetFriends);
                        let result = friendships_yielder
                            .r#yield(UsersResponse::from_response(users_response::Response::InternalServerError(
                                error)))
                            .await;
                        if let Err(err) = result {
                            log::error!("There was an error yielding the error to the friendships generator: {:?}", err);
                        };
                        return Ok(friendships_generator);
                    };
                tokio::spawn(async move {
                    let mut users = Users::default();

                    let friends_stream_page_size =
                        context.server_context.friends_stream_page_size as usize;

                    while let Some(friendship) = friendship.next().await {
                        users.users.push(build_user(friendship, user_id.clone()));
                        if users.users.len() == friends_stream_page_size {
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
                    if !users.users.is_empty() {
                        let result = friendships_yielder
                            .r#yield(UsersResponse::from_response(
                                users_response::Response::Users(users),
                            ))
                            .await;
                        if let Err(err) = result {
                            log::error!("There was an error yielding the response to the friendships generator: {:?}", err);
                        };
                    }
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
                record_error_response_code(err.clone().into(), Procedure::GetRequestEvents);
                return Ok(err.into());
            }
            Ok(user_id) => {
                let social_id = user_id.social_id.clone();
                log::info!("Getting requests events for user: {}", social_id);

                let Some(repos) = context.server_context.db.db_repos.clone() else {
                    log::error!("Get request events > Db repositories > `repos` is None.");
                    let error = InternalServerError { message: "".to_owned() };
                    record_error_response_code(error.clone().into(), Procedure::GetRequestEvents);

                    return Ok(RequestEventsResponse::from_response(
                        request_events_response::Response::InternalServerError(error)));
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
                        let error = InternalServerError {
                            message: "".to_owned(),
                        };
                        record_error_response_code(
                            error.clone().into(),
                            Procedure::GetRequestEvents,
                        );

                        Ok(RequestEventsResponse::from_response(
                            request_events_response::Response::InternalServerError(error),
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
            let error = UnauthorizedError{ message: "`auth_token` was not provided".to_owned() };
            record_error_response_code(error.clone().into(), Procedure::UpdateFriendshipEvent);
            return Ok(UpdateFriendshipResponse::from_response(
                update_friendship_response::Response::UnauthorizedError(
                    error
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
                record_error_response_code(err.clone().into(), Procedure::UpdateFriendshipEvent);
                return Ok(err.into());
            }
            Ok(user_id) => {
                // mapping with no colateral effect
                let event_payload = update_request_as_event_payload(request.clone());

                match event_payload {
                    Err(err) => {
                        record_error_response_code(
                            err.clone().into(),
                            Procedure::UpdateFriendshipEvent,
                        );
                        return Ok(err.into());
                    }
                    Ok(event_payload) => {
                        // TODO: Check if this is necessary as the token was already parsed
                        let token = get_synapse_token(request.clone());

                        match token {
                            Err(err) => {
                                record_error_response_code(
                                    err.clone().into(),
                                    Procedure::UpdateFriendshipEvent,
                                );
                                return Ok(err.into());
                            }
                            Ok(token) => {
                                let friendship_update_response = handle_friendship_update(
                                    token,
                                    event_payload,
                                    context.server_context.clone(),
                                    user_id.clone().social_id,
                                )
                                .await;

                                match friendship_update_response {
                                    Err(err) => {
                                        record_error_response_code(
                                            err.clone().into(),
                                            Procedure::UpdateFriendshipEvent,
                                        );
                                        return Ok(err.into());
                                    }
                                    Ok(friendship_update_response) => {
                                        let created_at = SystemTime::now()
                                            .duration_since(UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs()
                                            as i64;
                                        let update_response = event_response_as_update_response(
                                            request.clone(),
                                            friendship_update_response,
                                            created_at,
                                        );

                                        match update_response {
                                            Err(err) => {
                                                record_error_response_code(
                                                    err.clone().into(),
                                                    Procedure::UpdateFriendshipEvent,
                                                );
                                                return Ok(err.into());
                                            }
                                            Ok(update_response) => {
                                                let publisher =
                                                    context.server_context.redis_publisher.clone();
                                                if let Some(event) = request.clone().event {
                                                    tokio::spawn(async move {
                                                        if let Ok(
                                                            update_friendship_payload_as_event,
                                                        ) = update_friendship_payload_as_event(
                                                            event,
                                                            user_id.social_id.as_str(),
                                                            created_at,
                                                        ) {
                                                            publisher
                                                                .publish(update_friendship_payload_as_event)
                                                                .await;
                                                        } else {
                                                            log::error!("[RPC] There was an error parsing from friendship payload to event")
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
                record_error_response_code(
                    err.clone().into(),
                    Procedure::SubscribeFriendshipEventsUpdates,
                );

                let result = friendships_yielder.r#yield(err.into()).await;
                if let Err(err) = result {
                    log::error!("[RPC] There was an error yielding the error to the subscribe friendships generator: {:?}", err);
                };
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

/// Retrieves the User Id associated with the given Authentication Token.
///
/// If an authentication token was provided in the request, gets the
/// user id from the token and returns it as a `Result<UserId>`. If no
/// authentication token was provided, returns a `Err(CommonError::Unauthorized)`
/// error.
pub async fn get_user_id_from_request(
    request: &Payload,
    synapse: SynapseComponent,
    users_cache: Arc<Mutex<UsersCacheComponent>>,
) -> Result<UserId, CommonError> {
    match request.synapse_token.clone() {
        // If an authentication token was provided, get the user id from the token
        Some(token) => get_user_id_from_token(synapse.clone(), users_cache.clone(), &token)
            .await
            .map_err(|err| {
                log::error!("[RPC] Get user id from request > Error {err}");
                err
            }),
        // If no authentication token was provided, return an Unauthorized error.
        None => {
            log::error!("[RPC] Get user id from request > `synapse_token` is None.");
            Err(CommonError::Unauthorized(
                "`synapse_token` was not provided".to_owned(),
            ))
        }
    }
}

/// Filters out the friend of the authenticated user based on the provided `user_id`.
///
/// * `friendship` - A `Friendship` struct representing the friendship between the two users.
/// * `user_id` - The id of the authenticated user.
fn build_user(friendship: Friendship, user_id: UserId) -> User {
    let address1: String = friendship.address_1;
    let address2: String = friendship.address_2;
    match address1.eq_ignore_ascii_case(&user_id.social_id) {
        true => User { address: address2 },
        false => User { address: address1 },
    }
}
