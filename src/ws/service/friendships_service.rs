use std::sync::Arc;

use dcl_rpc::stream_protocol::Generator;
use futures_util::StreamExt;
use tokio::sync::Mutex;

use crate::{
    components::{synapse::SynapseComponent, users_cache::UsersCacheComponent},
    entities::{
        friendship_history::FriendshipRequestEvent, friendships::FriendshipRepositoryImplementation,
    },
    friendship_event_payload,
    ports::users_cache::{get_user_id_from_token, UserId},
    ws::service::error::FriendshipsServiceErrorResponse,
    ws::{app::SocialContext, service::error::FriendshipsServiceError},
    FriendshipsServiceServer, Payload, RequestEvents, RequestResponse, Requests,
    ServerStreamResponse, SubscribeFriendshipEventsUpdatesResponse, UpdateFriendshipPayload,
    UpdateFriendshipResponse, User, Users,
};

#[derive(Debug)]
pub struct MyFriendshipsService {}

#[async_trait::async_trait]
impl FriendshipsServiceServer<SocialContext> for MyFriendshipsService {
    #[tracing::instrument(name = "RPC SERVER > Get Friends Generator", skip(request, context))]
    async fn get_friends(
        &self,
        request: Payload,
        context: Arc<SocialContext>,
    ) -> ServerStreamResponse<Users> {
        // Get user id with the given Authentication Token.
        let user_id = get_user_id_from_request(
            &request,
            context.synapse.clone(),
            context.users_cache.clone(),
        )
        .await;

        match user_id {
            Ok(user_id) => {
                let social_id = user_id.social_id.clone();
                log::info!("Getting all friends for user: {}", social_id);
                // Look for users friends
                let mut friendship = match context.db.db_repos.clone() {
                    Some(repos) => {
                        let friendship = repos
                            .friendships
                            .get_user_friends_stream(&user_id.social_id, true)
                            .await;
                        match friendship {
                            // TODO: Handle get friends stream query response error.
                            Err(err) => {
                                log::error!(
                                    "Get Friends > Get User Friends Stream > Error: {err}."
                                );
                                todo!()
                            }
                            Ok(it) => it,
                        }
                    }
                    // TODO: Handle repos None.
                    None => {
                        log::error!("Get Friends > Db Repositories > `repos` is None.");
                        todo!()
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

                                // TODO: Move this value (5) to a Env Variable, Config or sth like that
                                if users_len == 5 {
                                    generator_yielder.r#yield(users).await.unwrap();
                                    users = Users::default();
                                }
                            }
                            None => {
                                generator_yielder.r#yield(users).await.unwrap();
                                break;
                            }
                        }
                    }
                });

                log::info!("Returning generator for all friends for user {}", social_id);
                generator
            }
            Err(_err) => {
                // TODO: Handle error when trying to get User Id.
                log::error!("Get Friends > Get User ID from Token > Error.");
                todo!()
            }
        }
    }
    #[tracing::instrument(name = "RPC SERVER > Get Request Events", skip(request, context))]
    async fn get_request_events(
        &self,
        request: Payload,
        context: Arc<SocialContext>,
    ) -> RequestEvents {
        // Get user id with the given Authentication Token.
        let user_id = get_user_id_from_request(
            &request,
            context.synapse.clone(),
            context.users_cache.clone(),
        )
        .await;

        match user_id {
            Ok(user_id) => {
                // Look for users requests
                match context.db.db_repos.clone() {
                    Some(repos) => {
                        let requests = repos
                            .friendship_history
                            .get_user_pending_request_events(&user_id.social_id)
                            .await;
                        match requests {
                            // TODO: Handle get user requests query response error.
                            Err(err) => {
                                log::debug!("Get Friends > Get User Requests > Error: {err}.");
                                todo!()
                            }
                            Ok(requests) => map_request_events(requests, user_id.social_id),
                        }
                    }
                    // TODO: Handle repos None.
                    None => {
                        log::debug!("Get Friends > Db Repositories > `repos` is None.");
                        todo!()
                    }
                }
            }
            Err(_err) => {
                // TODO: Handle error when trying to get User Id.
                log::debug!("Get Friends > Get User ID from Token > Error.");
                todo!()
            }
        }
    }

    #[tracing::instrument(name = "RPC SERVER > Update Friendship Event", skip(request, context))]
    async fn update_friendship_event(
        &self,
        request: UpdateFriendshipPayload,
        context: Arc<SocialContext>,
    ) -> UpdateFriendshipResponse {
        let subscriptions = context.friendships_events_subscriptions.clone();
        let user_id_to = get_user_id_to(request.clone());
        let event_update = to_update(request);
        // Notify local listeners
        tokio::spawn(async move {
            let subs = subscriptions.read().await;
            match user_id_to {
                Ok(user_to) => {
                    match event_update {
                        Some(event) => {
                            if let Some(generator) = subs.get(&user_to) {
                                if generator.r#yield(event).await.is_err() {
                                    log::error!("Event Update received > Couldn't send update to subscriptors");
                                }
                            }
                        }
                        None => {}
                    }
                }
                Err(_err) => {}
            }
        });

        // Notify channel

        todo!()
    }

    #[tracing::instrument(
        name = "RPC SERVER > Subscribe to friendship updates",
        skip(request, context)
    )]
    async fn subscribe_friendship_events_updates(
        &self,
        request: Payload,
        context: Arc<SocialContext>,
    ) -> ServerStreamResponse<SubscribeFriendshipEventsUpdatesResponse> {
        // Get user id with the given Authentication Token.
        let user_id = get_user_id_from_request(
            &request,
            context.synapse.clone(),
            context.users_cache.clone(),
        )
        .await;
        let (generator, generator_yielder) = Generator::create();

        // Attach generator to the context by user_id
        match user_id {
            Ok(user_id) => {
                context
                    .friendships_events_subscriptions
                    .write()
                    .await
                    .insert(user_id.social_id, generator_yielder.clone());
            }
            Err(_err) => {
                // TODO: Handle error when trying to get User Id.
                log::error!("Subscribe friendship event updates > Get User ID from Token > Error.");
                todo!()
            }
        }
        // TODO: Remove generator from map when user has disconnected
        generator
    }
}

fn to_update(
    _request: UpdateFriendshipPayload,
) -> Option<SubscribeFriendshipEventsUpdatesResponse> {
    todo!()
}

fn get_user_id_to(
    request: UpdateFriendshipPayload,
) -> Result<String, FriendshipsServiceErrorResponse> {
    let address_to = if let Some(body) = request.event {
        match body.body {
            Some(friendship_event_payload::Body::Request(request)) => {
                request
                    .user
                    .ok_or(FriendshipsServiceError::InternalServerError)?
                    .address
            }
            Some(friendship_event_payload::Body::Accept(accept)) => {
                accept
                    .user
                    .ok_or(FriendshipsServiceError::InternalServerError)?
                    .address
            }
            Some(friendship_event_payload::Body::Reject(reject)) => {
                reject
                    .user
                    .ok_or(FriendshipsServiceError::InternalServerError)?
                    .address
            }
            Some(friendship_event_payload::Body::Cancel(cancel)) => {
                cancel
                    .user
                    .ok_or(FriendshipsServiceError::InternalServerError)?
                    .address
            }
            Some(friendship_event_payload::Body::Delete(delete)) => {
                delete
                    .user
                    .ok_or(FriendshipsServiceError::InternalServerError)?
                    .address
            }
            None => return Err(FriendshipsServiceError::InternalServerError.into()),
        }
    } else {
        return Err(FriendshipsServiceError::InternalServerError.into());
    };
    Ok(address_to)
}

/// Retrieve the User Id associated with the given Authentication Token.
///
/// If an authentication token was provided in the request, this function gets the
/// user id from the token and returns it as a `Result<UserId, Error>`. If no
/// authentication token was provided, this function returns a `Unauthorized`
/// error.
///
/// * `request` -
/// * `context` -
async fn get_user_id_from_request(
    request: &Payload,
    synapse: SynapseComponent,
    users_cache: Arc<Mutex<UsersCacheComponent>>,
) -> Result<UserId, FriendshipsServiceErrorResponse> {
    match request.synapse_token.clone() {
        // If an authentication token was provided, get the user id from the token
        Some(token) => get_user_id_from_token(synapse.clone(), users_cache.clone(), &token)
            .await
            .map_err(|_err| -> FriendshipsServiceErrorResponse {
                FriendshipsServiceError::InternalServerError.into()
            }),
        // If no authentication token was provided, return an Unauthorized error.
        None => {
            log::debug!("Get Friends > Get User ID from Token > `synapse_token` is None.");
            Err(FriendshipsServiceError::Unauthorized.into())
        }
    }
}

/// Maps a list of `FriendshipRequestEvents` to a `RequestEvents` struct.
///
/// * `requests` - A vector of `FriendshipRequestEvents` to map to `RequestResponse` struct.
/// * `user_id` - The id of the auth user.
pub fn map_request_events(requests: Vec<FriendshipRequestEvent>, user_id: String) -> RequestEvents {
    let mut outgoing_requests: Vec<RequestResponse> = Vec::new();
    let mut incoming_requests: Vec<RequestResponse> = Vec::new();

    // Iterate through each friendship request event
    for request in requests {
        // Get the user id of the acting user for the request
        let acting_user_id = request.acting_user.clone();

        // Determine the address of the other user involved in the request event
        let address = if request.address_1.eq_ignore_ascii_case(&user_id) {
            request.address_2.clone()
        } else {
            request.address_1.clone()
        };

        // Get the message (if any) associated with the request
        let message = request
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.message.clone());

        let request_response = RequestResponse {
            user: Some(User { address }),
            created_at: request.timestamp.timestamp(),
            message,
        };

        if acting_user_id.eq_ignore_ascii_case(&user_id) {
            // If the acting user is the same as the user ID, then the request is outgoing
            outgoing_requests.push(request_response);
        } else {
            // Otherwise, the request is incoming
            incoming_requests.push(request_response);
        }
    }

    // Return a RequestEvents struct containing the incoming and outgoing request lists
    RequestEvents {
        outgoing: Some(Requests {
            total: outgoing_requests.len() as i64,
            items: outgoing_requests,
        }),
        incoming: Some(Requests {
            total: incoming_requests.len() as i64,
            items: incoming_requests,
        }),
    }
}
