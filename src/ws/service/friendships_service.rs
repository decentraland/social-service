use std::sync::Arc;

use dcl_rpc::stream_protocol::Generator;
use futures_util::StreamExt;

use crate::{
    components::database::DatabaseComponentImplementation,
    entities::friendships::FriendshipRepositoryImplementation,
    ws::{
        app::SocialContext,
        service::{
            error::FriendshipsServiceError,
            friendship_ws_types::{
                FriendshipEventWs, FriendshipPortsWs, FriendshipStatusWs, RoomInfoWs,
            },
            helpers::{get_last_history, store_message_in_synapse_room, update_friendship_status},
        },
    },
    FriendshipsServiceServer, Payload, RequestEvents, ServerStreamResponse,
    SubscribeFriendshipEventsUpdatesResponse, UpdateFriendshipPayload, UpdateFriendshipResponse,
    User, Users,
};

use super::{
    error::FriendshipsServiceErrorResponse,
    helpers::{get_friendship, get_user_id_from_request, map_request_events},
};

pub struct EventResponse {
    pub event_id: String,
}

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
        // Get user id with the given Authentication Token.
        let user_id = get_user_id_from_request(
            &request.clone().auth_token.unwrap(),
            context.synapse.clone(),
            context.users_cache.clone(),
        )
        .await;

        // Process rooom event as in
        match user_id {
            Ok(user_id) => {
                let _result = process_room_event(request, context, user_id.social_id);
            }
            Err(_) => todo!(),
        }

        // Return Response

        todo!()
    }

    #[tracing::instrument(
        name = "RPC SERVER > Subscribe to friendship updates",
        skip(_request, _context)
    )]
    async fn subscribe_friendship_events_updates(
        &self,
        _request: Payload,
        _context: Arc<SocialContext>,
    ) -> ServerStreamResponse<SubscribeFriendshipEventsUpdatesResponse> {
        todo!()
    }
}

async fn process_room_event(
    request: UpdateFriendshipPayload,
    context: Arc<SocialContext>,
    user_id: String,
) -> Result<EventResponse, FriendshipsServiceErrorResponse> {
    // TODO: Get current event
    let current_event = FriendshipEventWs::ACCEPT;

    // TODO: Get second_user from event.user.address
    let acting_user = user_id;
    let second_user = "".to_string();

    // Get the friendship info
    let db_repos = &context.db.clone().db_repos.unwrap();
    let friendships_repository = &db_repos.friendships;
    let friendship = get_friendship(friendships_repository, &acting_user, &second_user).await?;

    // TODO: If there is no existing Friendship and the event type is REQUEST, create a new room.
    // TODO: If there is no existing Friendship and it is not a REQUEST Event, return an Invalid Action error.
    let (friendship, synapse_room_id) = match friendship {
        Some(friendship) => (Some(friendship), ""), // TODO: friendship.synapse_room_id
        None => {
            if current_event == FriendshipEventWs::REQUEST {
                // TODO: Create room
                let synapse_room_id = "";
                (None, synapse_room_id)
            } else {
                return Err(FriendshipsServiceError::InternalServerError.into());
            }
        }
    };

    //  Get the last status from the database to later validate if the current action is valid.
    let friendship_history_repository = &db_repos.friendship_history;
    let _last_history = get_last_history(friendship_history_repository, &friendship).await?;

    // TODO: Validate if the new status that is trying to be set is valid. If it's invalid or it has not changed, return here.
    let status = FriendshipStatusWs::Friends;

    // Start a database transaction.
    let friendship_ports = FriendshipPortsWs {
        db: &context.db,
        friendships_repository: &db_repos.friendships,
        friendship_history_repository: &db_repos.friendship_history,
    };
    let transaction = match friendship_ports.db.start_transaction().await {
        Ok(tx) => tx,
        Err(error) => {
            log::error!("Couldn't start transaction to store friendship update {error}");
            todo!()
        }
    };

    // Update the friendship accordingly in the database. This means creating an entry in the friendships table or updating the is_active column.
    // TODO: RoomInfoWs
    let room_info = RoomInfoWs {
        room_event: current_event,
        room_message_body: None,
        room_id: synapse_room_id,
    };
    let _transaction = update_friendship_status(
        &friendship,
        &acting_user,
        &second_user,
        status,
        room_info,
        friendship_ports,
        transaction,
    )
    .await?;

    // If it's a friendship request event and the request contains a message, send a message event to the given room.
    let token = request.auth_token.unwrap().synapse_token.unwrap();
    store_message_in_synapse_room(
        &token,
        synapse_room_id,
        current_event,
        None,
        &context.synapse,
    )
    .await?;

    // TODO: Store the friendship event in the given room.

    // TODO: End the database transaction.

    // TODO: Return the result.
    todo!()
}
