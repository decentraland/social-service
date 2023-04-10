use std::sync::Arc;

use dcl_rpc::stream_protocol::Generator;
use futures_util::StreamExt;

use crate::{
    api::routes::v1::{error::CommonError, friendships::errors::FriendshipsError},
    entities::{
        friendship_history::FriendshipRequestEvents,
        friendships::FriendshipRepositoryImplementation,
    },
    ports::users_cache::{get_user_id_from_token, UserId},
    ws::app::SocialContext,
    FriendshipsServiceServer, Payload, RequestEvents, RequestResponse, Requests,
    ServerStreamResponse, SubscribeFriendshipEventsUpdatesResponse, UpdateFriendshipPayload,
    UpdateFriendshipResponse, User, Users,
};

pub struct MyFriendshipsService {}

#[async_trait::async_trait]
impl FriendshipsServiceServer<SocialContext> for MyFriendshipsService {
    async fn get_friends(
        &self,
        request: Payload,
        context: Arc<SocialContext>,
    ) -> ServerStreamResponse<Users> {
        // Get user id with the given Authentication Token.
        let user_id = get_user_id_from_request(&request, &context).await;

        match user_id {
            Ok(user_id) => {
                // Look for users friends
                let mut friendship = match context.app_components.db.db_repos.clone() {
                    Some(repos) => {
                        let friendship = repos
                            .friendships
                            .get_user_friends_stream(&user_id.social_id, true)
                            .await;
                        match friendship {
                            // TODO: Handle get friends stream query response error.
                            Err(err) => {
                                log::debug!(
                                    "Get Friends > Get User Friends Stream > Error: {err}."
                                );
                                // Err(FriendshipsError::CommonError(CommonError::Unknown)),
                                todo!()
                            }
                            Ok(it) => it,
                        }
                    }
                    // TODO: Handle repos None.
                    None => {
                        // Err(FriendshipsError::CommonError(CommonError::NotFound))
                        log::debug!("Get Friends > Db Repositories > `repos` is None.");
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

                generator
            }
            Err(err) => {
                // TODO: Handle error when trying to get User Id.
                log::debug!("Get Friends > Get User ID from Token > Error: {err}.");
                // Err(FriendshipsError::CommonError(CommonError::err)),
                todo!()
            }
        }
    }
    async fn get_request_events(
        &self,
        request: Payload,
        context: Arc<SocialContext>,
    ) -> RequestEvents {
        // Get user id with the given Authentication Token.
        let user_id = get_user_id_from_request(&request, &context).await;

        match user_id {
            Ok(user_id) => {
                // Look for users requests
                let requests = match context.app_components.db.db_repos.clone() {
                    Some(repos) => {
                        let requests = repos
                            .friendship_history
                            .get_user_request_events(&user_id.social_id)
                            .await;
                        match requests {
                            // TODO: Handle get user requests query response error.
                            Err(err) => {
                                log::debug!("Get Friends > Get User Requests > Error: {err}.");
                                // Err(FriendshipsError::CommonError(CommonError::Unknown)),
                                todo!()
                            }
                            Ok(it) => map_request_events(it, user_id.social_id),
                        }
                    }
                    // TODO: Handle repos None.
                    None => {
                        // Err(FriendshipsError::CommonError(CommonError::NotFound))
                        log::debug!("Get Friends > Db Repositories > `repos` is None.");
                        todo!()
                    }
                };
                requests
            }
            Err(err) => {
                // TODO: Handle error when trying to get User Id.
                log::debug!("Get Friends > Get User ID from Token > Error: {err}.");
                // Err(FriendshipsError::CommonError(CommonError::err)),
                todo!()
            }
        }
    }

    async fn update_friendship_event(
        &self,
        _request: UpdateFriendshipPayload,
        _context: Arc<SocialContext>,
    ) -> UpdateFriendshipResponse {
        todo!()
    }

    async fn subscribe_friendship_events_updates(
        &self,
        _request: Payload,
        _context: Arc<SocialContext>,
    ) -> ServerStreamResponse<SubscribeFriendshipEventsUpdatesResponse> {
        todo!()
    }
}

/// Retrieve the User Id associated with the given Authentication Token.
async fn get_user_id_from_request(
    request: &Payload,
    context: &Arc<SocialContext>,
) -> Result<UserId, FriendshipsError> {
    match request.synapse_token.clone() {
        // Get User Id
        Some(token) => get_user_id_from_token(context.app_components.clone(), &token)
            .await
            .map_err(FriendshipsError::CommonError),
        // If no authentication token was provided, return an Unauthorized error.
        None => {
            log::debug!("Get Friends > Get User ID from Token > `synapse_token` is None.");
            Err(FriendshipsError::CommonError(CommonError::Unauthorized))
        }
    }
}

fn map_request_events(requests: Vec<FriendshipRequestEvents>, user_id: String) -> RequestEvents {
    let mut outgoing_requests: Vec<RequestResponse> = Vec::new();
    let mut incoming_requests: Vec<RequestResponse> = Vec::new();

    for request in requests {
        let acting_user_id = request.acting_user.clone();

        let address = if request.address_1 == user_id {
            request.address_2.clone()
        } else {
            request.address_1.clone()
        };
        let message = request
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.message.clone());

        let request_response = RequestResponse {
            user: Some(User { address }),
            created_at: request.timestamp.timestamp(),
            message,
        };

        if acting_user_id == user_id {
            outgoing_requests.push(request_response);
        } else {
            incoming_requests.push(request_response);
        }
    }

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
