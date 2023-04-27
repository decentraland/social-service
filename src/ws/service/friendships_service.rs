use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use dcl_rpc::stream_protocol::Generator;
use futures_util::StreamExt;

use crate::{
    components::notifications::{ChannelPublisher, RedisChannelPublisher},
    entities::friendships::FriendshipRepositoryImplementation,
    friendships::FriendshipsServiceServer,
    friendships::Payload,
    friendships::RequestEvents,
    friendships::ServerStreamResponse,
    friendships::SubscribeFriendshipEventsUpdatesResponse,
    friendships::UpdateFriendshipResponse,
    friendships::User,
    friendships::Users,
    friendships::{FriendshipEventPayload, UpdateFriendshipPayload},
    notifications::Event,
    ws::app::SocialContext,
};

use super::{
    friendship_event_updates::handle_friendship_update,
    mapper::{
        event_response_as_update_response, friendship_requests_as_request_events,
        update_friendship_payload_as_event,
    },
    synapse_handler::get_user_id_from_request,
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
                            // TODO: Handle get friends stream query response error. Ticket #81
                            Err(err) => {
                                log::error!(
                                    "Get Friends > Get User Friends Stream > Error: {err}."
                                );
                                todo!()
                            }
                            Ok(it) => it,
                        }
                    }
                    // TODO: Handle repos None. Ticket #81
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
                // TODO: Handle error when trying to get User Id. Ticket #81
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
                            // TODO: Handle get user requests query response error. Ticket #81
                            Err(err) => {
                                log::debug!("Get Friends > Get User Requests > Error: {err}.");
                                todo!()
                            }
                            Ok(requests) => {
                                friendship_requests_as_request_events(requests, user_id.social_id)
                            }
                        }
                    }
                    // TODO: Handle repos None. Ticket #81
                    None => {
                        log::error!("Get Friends > Db Repositories > `repos` is None.");
                        todo!()
                    }
                }
            }
            Err(_err) => {
                // TODO: Handle error when trying to get User Id. Ticket #81
                log::error!("Get Friends > Get User ID from Token > Error.");
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
        // TODO: Do not `unwrap`, handle error instead. Ticket #81
        let user_id = get_user_id_from_request(
            &request.clone().auth_token.unwrap(),
            context.synapse.clone(),
            context.users_cache.clone(),
        )
        .await;

        match user_id {
            Ok(user_id) => {
                let process_room_event_response = handle_friendship_update(
                    request.clone(),
                    context.clone(),
                    user_id.social_id.clone(),
                )
                .await;

                if let Ok(event_response) = process_room_event_response {
                    if let Ok(res) =
                        event_response_as_update_response(request.clone(), event_response)
                    {
                        // TODO: fix this
                        let created_at = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64;

                        let publisher = context.redis_publisher.clone();
                        if let Some(event) = request.clone().event {
                            tokio::spawn(async move {
                                log::debug!(
                                    "About to publish on pub/sub system about new event: {:?}",
                                    event.clone()
                                );
                                publish_on_channel(event, publisher, user_id.social_id, created_at)
                                    .await;
                            });
                        };

                        res
                    } else {
                        // TODO: Ticket #81
                        todo!()
                    }
                } else {
                    // TODO: Ticket #81
                    todo!()
                }
            }
            Err(_) => {
                // TODO: Handle error when trying to get User Id. Ticket #81
                log::error!("Update Frienship Event > Get User ID from Token > Error.");
                todo!()
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
                log::debug!(
                    "About to add new generator for user_id: {}",
                    user_id.social_id
                );
                context
                    .friendships_events_subscriptions
                    .write()
                    .await
                    .insert(user_id.social_id.to_lowercase(), generator_yielder.clone());
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

async fn publish_on_channel(
    payload: FriendshipEventPayload,
    publisher: Arc<RedisChannelPublisher>,
    actor: String,
    created_at: i64,
) {
    let event: Event = update_friendship_payload_as_event(payload, actor, created_at);
    publisher.publish(event).await;
}
