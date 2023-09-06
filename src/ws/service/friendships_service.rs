use std::{
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use dcl_rpc::{
    rpc_protocol::RemoteErrorResponse,
    {service_module_definition::ProcedureContext, stream_protocol::Generator},
};
use futures_util::StreamExt;
use prost::Message;
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
        request_events_response, update_friendship_response, users_response, BadRequestError,
        FriendshipsServiceServer, InternalServerError, MutualFriendsPayload, Payload,
        RequestEventsResponse, ServerStreamResponse, SubscribeFriendshipEventsUpdatesResponse,
        UnauthorizedError, UpdateFriendshipPayload, UpdateFriendshipResponse, User, Users,
        UsersResponse,
    },
    ws::{
        app::{SocialContext, SocialTransportContext},
        metrics::Procedure,
    },
};

use super::{
    friendship_event_updates::handle_friendship_update,
    mapper::{
        event::{
            event_response_as_update_response, friendship_requests_as_request_events_response,
            parse_event_payload_to_friendship_event, update_friendship_payload_as_event,
            update_request_as_event_payload,
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
        let start_time = Instant::now();
        let metrics = context.server_context.metrics.clone();
        metrics
            .clone()
            .record_in_procedure_call_size(Procedure::GetFriends, &request);

        let request_user_id = get_user_id_from_request(
            &request,
            context.server_context.synapse.clone(),
            context.server_context.users_cache.clone(),
        )
        .await;

        let (friendships_generator, friendships_yielder) = Generator::create();

        let Some(repos) = context.server_context.db.db_repos.clone() else {
            log::error!("[RPC] Get friends > Db repositories > `repos` is None.");
            let error = InternalServerError {
                message: "An error occurred while getting the friendships".to_owned(),
            };
            metrics.record_procedure_call_and_duration_and_out_size(
                Some(error.clone().into()),
                Procedure::GetFriends,
                start_time,
                error.encoded_len(),
            );

            let result = friendships_yielder
                .r#yield(UsersResponse::from_response(
                    users_response::Response::InternalServerError(error),
                ))
                .await;
            if let Err(err) = result {
                log::error!("[RPC] There was an error yielding the error to the friendships generator: {:?}", err);
            };
            return Ok(friendships_generator);
        };

        match request_user_id {
            Err(err) => {
                let error_response: UsersResponse = err.clone().into();
                metrics.record_procedure_call_and_duration_and_out_size(
                    Some(err.clone().into()),
                    Procedure::GetFriends,
                    start_time,
                    error_response.encoded_len(),
                );
                let result = friendships_yielder.r#yield(error_response).await;
                if let Err(err) = result {
                    log::error!(
                        "[RPC] There was an error yielding the error to the friendships generator: {:?}",
                        err
                    );
                };
            }
            Ok(user_id) => {
                let social_id = user_id.social_id.clone();
                log::info!("[RPC] Getting all friends for user: {}", social_id);

                let Ok(mut friendship) = repos
                    .friendships
                    .get_user_friends_stream(&user_id.social_id, true)
                    .await
                else {
                    log::error!(
                            "[RPC] Get friends > Get user friends stream > Error: There was an error accessing to the friendships repository."
                        );
                    let error = InternalServerError {
                        message: "An error occurred while sending the response to the stream"
                            .to_owned(),
                    };
                    metrics.record_procedure_call_and_duration_and_out_size(
                        Some(error.clone().into()),
                        Procedure::GetFriends,
                        start_time,
                        error.encoded_len(),
                    );

                    let result = friendships_yielder
                        .r#yield(UsersResponse::from_response(
                            users_response::Response::InternalServerError(error),
                        ))
                        .await;
                    if let Err(err) = result {
                        log::error!("[RPC] There was an error yielding the error to the friendships generator: {:?}", err);
                    };
                    return Ok(friendships_generator);
                };
                let metrics_clone = metrics.clone();
                tokio::spawn(async move {
                    let mut users = Users::default();

                    let friends_stream_page_size =
                        context.server_context.friends_stream_page_size as usize;

                    while let Some(friendship) = friendship.next().await {
                        users.users.push(build_user(friendship, user_id.clone()));
                        if users.users.len() == friends_stream_page_size {
                            let response = UsersResponse::from_response(
                                users_response::Response::Users(users.clone()),
                            );
                            metrics_clone.record_out_procedure_call_size(
                                None,
                                Procedure::GetFriends,
                                response.encoded_len(),
                            );
                            let result = friendships_yielder.r#yield(response).await;
                            if let Err(err) = result {
                                log::error!("[RPC] There was an error yielding the response to the friendships generator: {:?}", err);
                                break;
                            };
                            users = Users::default();
                        }
                    }
                    if !users.users.is_empty() {
                        let response =
                            UsersResponse::from_response(users_response::Response::Users(users));
                        metrics_clone.record_out_procedure_call_size(
                            None,
                            Procedure::GetFriends,
                            response.encoded_len(),
                        );
                        let result = friendships_yielder.r#yield(response).await;
                        if let Err(err) = result {
                            log::error!("[RPC] There was an error yielding the response to the friendships generator: {:?}", err);
                        };
                    }
                });

                metrics.record_procedure_call_and_duration(None, Procedure::GetFriends, start_time);

                log::info!(
                    "[RPC] Returning generator for all friends for user {}",
                    social_id
                );
            }
        }
        Ok(friendships_generator)
    }

    #[tracing::instrument(
        name = "RPC SERVER > Get Mutual Friends Generator",
        skip(request, context)
    )]
    async fn get_mutual_friends(
        &self,
        request: MutualFriendsPayload,
        context: ProcedureContext<SocialContext>,
    ) -> Result<ServerStreamResponse<UsersResponse>, RPCFriendshipsServiceError> {
        let start_time = Instant::now();
        let metrics = context.server_context.metrics.clone();
        metrics
            .clone()
            .record_in_procedure_call_size(Procedure::GetFriends, &request);

        let (friendships_generator, friendships_yielder) = Generator::create();

        let Some(other_user) = request.user.clone() else {
            let error = BadRequestError {
                message: "`user` was not provided".to_owned(),
            };
            metrics.record_procedure_call_and_duration_and_out_size(
                Some(error.clone().into()),
                Procedure::GetMutualFriends,
                start_time,
                error.encoded_len(),
            );

            let result = friendships_yielder
                .r#yield(UsersResponse::from_response(
                    users_response::Response::BadRequestError(error),
                ))
                .await;
            if let Err(err) = result {
                log::error!("[RPC] There was an error yielding the error to the mutual friendships generator: {:?}", err);
            };
            return Ok(friendships_generator);
        };

        let Some(auth_token) = request.clone().auth_token.take() else {
            let error = UnauthorizedError {
                message: "`auth_token` was not provided".to_owned(),
            };
            metrics.record_procedure_call_and_duration_and_out_size(
                Some(error.clone().into()),
                Procedure::GetMutualFriends,
                start_time,
                error.encoded_len(),
            );

            let result = friendships_yielder
                .r#yield(UsersResponse::from_response(
                    users_response::Response::UnauthorizedError(error),
                ))
                .await;
            if let Err(err) = result {
                log::error!("[RPC] There was an error yielding the error to the mutual friendships generator: {:?}", err);
            };
            return Ok(friendships_generator);
        };

        let request_user_id = get_user_id_from_request(
            &auth_token,
            context.server_context.synapse.clone(),
            context.server_context.users_cache.clone(),
        )
        .await;

        let Some(repos) = context.server_context.db.db_repos.clone() else {
            log::error!("[RPC] Get mutual friends > Db repositories > `repos` is None.");
            let error = InternalServerError {
                message: "An error occurred while getting the mutual friendships".to_owned(),
            };
            metrics.record_procedure_call_and_duration_and_out_size(
                Some(error.clone().into()),
                Procedure::GetMutualFriends,
                start_time,
                error.encoded_len(),
            );

            let result = friendships_yielder
                .r#yield(UsersResponse::from_response(
                    users_response::Response::InternalServerError(error),
                ))
                .await;
            if let Err(err) = result {
                log::error!("[RPC] There was an error yielding the error to the mutual friendships generator: {:?}", err);
            };
            return Ok(friendships_generator);
        };

        match request_user_id {
            Err(err) => {
                let error_response: UsersResponse = err.clone().into();
                metrics.record_procedure_call_and_duration_and_out_size(
                    Some(err.clone().into()),
                    Procedure::GetMutualFriends,
                    start_time,
                    error_response.encoded_len(),
                );
                let result = friendships_yielder.r#yield(error_response).await;
                if let Err(err) = result {
                    log::error!(
                        "[RPC] There was an error yielding the error to the mutual friendships generator: {:?}",
                        err
                    );
                };
            }
            Ok(user_id) => {
                let social_id = user_id.social_id.clone();
                let other_id = other_user.address.clone();
                log::info!(
                    "[RPC] Getting all mutual friends for user: {} and {}",
                    social_id,
                    other_id
                );

                let Ok(mut friendship) = repos
                    .friendships
                    .clone()
                    .get_mutual_friends_stream(
                        user_id.social_id.clone().to_string(),
                        other_user.address.clone().to_string(),
                    )
                    .await
                else {
                    log::error!(
                            "[RPC] Get mutual friends > Get user friends stream > Error: There was an error accessing to the friendships repository."
                        );
                    let error = InternalServerError {
                        message: "An error occurred while sending the response to the stream"
                            .to_owned(),
                    };
                    metrics.record_procedure_call_and_duration_and_out_size(
                        Some(error.clone().into()),
                        Procedure::GetMutualFriends,
                        start_time,
                        error.encoded_len(),
                    );

                    let result = friendships_yielder
                        .r#yield(UsersResponse::from_response(
                            users_response::Response::InternalServerError(error),
                        ))
                        .await;
                    if let Err(err) = result {
                        log::error!("[RPC] There was an error yielding the error to the mutual friendships generator: {:?}", err);
                    };
                    return Ok(friendships_generator);
                };
                let metrics_clone = metrics.clone();
                tokio::spawn(async move {
                    let mut users: Users = Users::default();

                    let friends_stream_page_size =
                        context.server_context.friends_stream_page_size as usize;

                    while let Some(user_id) = friendship.next().await {
                        let current_user = User {
                            address: user_id.address,
                        };
                        users.users.push(current_user);
                        if users.users.len() == friends_stream_page_size {
                            let response = UsersResponse::from_response(
                                users_response::Response::Users(users.clone()),
                            );
                            metrics_clone.record_out_procedure_call_size(
                                None,
                                Procedure::GetMutualFriends,
                                response.encoded_len(),
                            );
                            let result = friendships_yielder.r#yield(response).await;
                            if let Err(err) = result {
                                log::error!("[RPC] There was an error yielding the response to the mutual friendships generator: {:?}", err);
                                break;
                            };
                            users = Users::default();
                        }
                    }
                    if !users.users.is_empty() {
                        let response = UsersResponse::from_response(
                            users_response::Response::Users(users.clone()),
                        );
                        metrics_clone.record_out_procedure_call_size(
                            None,
                            Procedure::GetMutualFriends,
                            response.encoded_len(),
                        );
                        let result = friendships_yielder.r#yield(response).await;
                        if let Err(err) = result {
                            log::error!("[RPC] There was an error yielding the response to the mutual friendships generator: {:?}", err);
                        };
                    }
                });

                metrics.record_procedure_call_and_duration(
                    None,
                    Procedure::GetMutualFriends,
                    start_time,
                );

                log::info!(
                    "[RPC] Returning generator for mutual friends for user {} and {}",
                    social_id,
                    other_id
                );
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
        let start_time = Instant::now();
        let metrics = context.server_context.metrics.clone();
        metrics.record_in_procedure_call_size(Procedure::GetRequestEvents, &request);

        let request_user_id = get_user_id_from_request(
            &request,
            context.server_context.synapse.clone(),
            context.server_context.users_cache.clone(),
        )
        .await;

        match request_user_id {
            Err(err) => {
                let error_response: RequestEventsResponse = err.clone().into();
                metrics.record_procedure_call_and_duration_and_out_size(
                    Some(err.into()),
                    Procedure::GetRequestEvents,
                    start_time,
                    error_response.encoded_len(),
                );
                return Ok(error_response);
            }
            Ok(user_id) => {
                let social_id = user_id.social_id.clone();
                log::info!("[RPC] Getting requests events for user: {}", social_id);

                let Some(repos) = context.server_context.db.db_repos.clone() else {
                    log::error!("[RPC] Get request events > Db repositories > `repos` is None.");
                    let error = InternalServerError {
                        message: "".to_owned(),
                    };
                    metrics.record_procedure_call_and_duration_and_out_size(
                        Some(error.clone().into()),
                        Procedure::GetRequestEvents,
                        start_time,
                        error.encoded_len(),
                    );

                    return Ok(RequestEventsResponse::from_response(
                        request_events_response::Response::InternalServerError(error),
                    ));
                };

                let requests = repos
                    .friendship_history
                    .get_user_pending_request_events(&user_id.social_id)
                    .await;

                match requests {
                    Err(err) => {
                        log::error!(
                            "[RPC] Get request events > Get user pending request events > Error: {err}."
                        );
                        let error = InternalServerError {
                            message: "".to_owned(),
                        };
                        metrics.record_procedure_call_and_duration_and_out_size(
                            Some(error.clone().into()),
                            Procedure::GetRequestEvents,
                            start_time,
                            error.encoded_len(),
                        );
                        Ok(RequestEventsResponse::from_response(
                            request_events_response::Response::InternalServerError(error),
                        ))
                    }
                    Ok(requests) => {
                        log::info!("Returning requests events for user {}", social_id);
                        let response = friendship_requests_as_request_events_response(
                            requests,
                            user_id.social_id,
                        );
                        metrics.record_procedure_call_and_duration_and_out_size(
                            None,
                            Procedure::GetRequestEvents,
                            start_time,
                            response.encoded_len(),
                        );
                        Ok(response)
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
        let start_time = Instant::now();
        let metrics = context.server_context.metrics.clone();
        metrics.record_in_procedure_call_size(Procedure::UpdateFriendshipEvent, &request);

        let Some(auth_token) = request.clone().auth_token.take() else {
            let error = UnauthorizedError {
                message: "`auth_token` was not provided".to_owned(),
            };
            metrics.record_procedure_call_and_duration_and_out_size(
                Some(error.clone().into()),
                Procedure::UpdateFriendshipEvent,
                start_time,
                error.encoded_len(),
            );

            return Ok(UpdateFriendshipResponse::from_response(
                update_friendship_response::Response::UnauthorizedError(error),
            ));
        };

        let request_user_id = get_user_id_from_request(
            &auth_token,
            context.server_context.synapse.clone(),
            context.server_context.users_cache.clone(),
        )
        .await;

        match request_user_id {
            Err(err) => {
                let error_response: UpdateFriendshipResponse = err.clone().into();
                metrics.record_procedure_call_and_duration_and_out_size(
                    Some(err.into()),
                    Procedure::UpdateFriendshipEvent,
                    start_time,
                    error_response.encoded_len(),
                );
                return Ok(error_response);
            }
            Ok(user_id) => {
                let event_payload = update_request_as_event_payload(request.clone());

                match event_payload {
                    Err(err) => {
                        let error_response: UpdateFriendshipResponse = err.clone().into();
                        metrics.record_procedure_call_and_duration_and_out_size(
                            Some(err.into()),
                            Procedure::UpdateFriendshipEvent,
                            start_time,
                            error_response.encoded_len(),
                        );
                        return Ok(error_response);
                    }
                    Ok(event_payload) => {
                        let token = get_synapse_token(request.clone());

                        match token {
                            Err(err) => {
                                let error_response: UpdateFriendshipResponse = err.clone().into();
                                metrics.record_procedure_call_and_duration_and_out_size(
                                    Some(err.into()),
                                    Procedure::UpdateFriendshipEvent,
                                    start_time,
                                    error_response.encoded_len(),
                                );
                                return Ok(error_response);
                            }
                            Ok(token) => {
                                // All the inserts are done here, no changes in the database after that call
                                let friendship_update_response = handle_friendship_update(
                                    token,
                                    event_payload,
                                    context.server_context.clone(),
                                    user_id.clone().social_id,
                                )
                                .await;

                                match friendship_update_response {
                                    Err(err) => {
                                        let error_response: UpdateFriendshipResponse =
                                            err.clone().into();
                                        metrics.record_procedure_call_and_duration_and_out_size(
                                            Some(err.into()),
                                            Procedure::UpdateFriendshipEvent,
                                            start_time,
                                            error_response.encoded_len(),
                                        );
                                        return Ok(error_response);
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

                                        let metrics_clone = Arc::clone(&metrics);
                                        match update_response {
                                            Err(err) => {
                                                let error_response: UpdateFriendshipResponse =
                                                    err.clone().into();
                                                metrics.record_procedure_call_and_duration_and_out_size(
                                                    Some(err.into()),
                                                    Procedure::UpdateFriendshipEvent,
                                                    start_time,
                                                    error_response.encoded_len(),
                                                );
                                                return Ok(error_response);
                                            }
                                            Ok(update_response) => {
                                                let publisher =
                                                    context.server_context.redis_publisher.clone();
                                                if let Some(event) = request.clone().event {
                                                    tokio::spawn(async move {
                                                        if let Ok(
                                                            update_friendship_payload_as_event,
                                                        ) = update_friendship_payload_as_event(
                                                            event.clone(),
                                                            user_id.social_id.as_str(),
                                                            created_at,
                                                        ) {
                                                            publisher
                                                                .publish(update_friendship_payload_as_event)
                                                                .await;

                                                            if let Some(event) =
                                                                parse_event_payload_to_friendship_event(
                                                                    event,
                                                                )
                                                            {
                                                              metrics_clone.record_friendship_event_updates_sent(
                                                                    event,
                                                                );
                                                            }
                                                        } else {
                                                            log::error!("[RPC] There was an error parsing from friendship payload to event")
                                                        }
                                                    });
                                                };
                                                metrics.record_procedure_call_and_duration_and_out_size(
                                                    None,
                                                    Procedure::UpdateFriendshipEvent,
                                                    start_time,
                                                    update_response.encoded_len(),
                                                );

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
        let start_time = Instant::now();
        let metrics = context.server_context.metrics.clone();
        metrics
            .record_in_procedure_call_size(Procedure::SubscribeFriendshipEventsUpdates, &request);

        let request_user_id = get_user_id_from_request(
            &request,
            context.server_context.synapse.clone(),
            context.server_context.users_cache.clone(),
        )
        .await;

        let (friendships_generator, friendships_yielder) = Generator::create();

        match request_user_id {
            Err(err) => {
                let error_response: SubscribeFriendshipEventsUpdatesResponse = err.clone().into();
                metrics.record_procedure_call_and_duration_and_out_size(
                    Some(err.clone().into()),
                    Procedure::SubscribeFriendshipEventsUpdates,
                    start_time,
                    error_response.encoded_len(),
                );
                let result = friendships_yielder.r#yield(error_response).await;
                if let Err(err) = result {
                    log::error!("[RPC] There was an error yielding the error to the subscribe friendships generator: {:?}", err);
                };
            }
            Ok(user_id) => {
                metrics.record_procedure_call_and_duration(
                    None,
                    Procedure::SubscribeFriendshipEventsUpdates,
                    start_time,
                );

                // Attach social_id to the context by transport_id
                let mut transport_context = context.server_context.transport_context.write().await;
                transport_context
                    .entry(context.transport_id)
                    .and_modify(|e| e.address = Address(user_id.social_id.to_string()))
                    .or_insert_with(|| {
                        log::warn!("This code should be unreachable");
                        // This should never happen
                        SocialTransportContext {
                            address: Address(user_id.social_id.to_string()),
                            connection_ts: Instant::now(),
                        }
                    });

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
async fn get_user_id_from_request(
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
