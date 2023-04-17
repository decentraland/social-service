use std::sync::Arc;

use dcl_rpc::stream_protocol::Generator;
use futures_util::StreamExt;
use sqlx::{Postgres, Transaction};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    api::routes::synapse::{
        errors::SynapseError,
        room_events::{FriendshipEvent, FriendshipStatus},
    },
    components::{
        database::{DatabaseComponent, DatabaseComponentImplementation},
        synapse::SynapseComponent,
        users_cache::UsersCacheComponent,
    },
    entities::{
        friendship_history::{
            FriendshipHistory, FriendshipHistoryRepository, FriendshipMetadata,
            FriendshipRequestEvent,
        },
        friendships::{Friendship, FriendshipRepositoryImplementation, FriendshipsRepository},
    },
    ports::users_cache::{get_user_id_from_token, UserId},
    ws::service::error::FriendshipsServiceErrorResponse,
    ws::{app::SocialContext, service::error::FriendshipsServiceError},
    FriendshipEventPayload, FriendshipsServiceServer, Payload, RequestEvents, RequestResponse,
    Requests, ServerStreamResponse, SubscribeFriendshipEventsUpdatesResponse,
    UpdateFriendshipPayload, UpdateFriendshipResponse, User, Users,
};

pub struct FriendshipPortsWs<'a> {
    db: &'a DatabaseComponent,
    friendships_repository: &'a FriendshipsRepository,
    friendship_history_repository: &'a FriendshipHistoryRepository,
}

pub struct RoomInfoWs<'a> {
    room_event: FriendshipEvent,
    room_message_body: Option<&'a str>,
    room_id: &'a str,
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
        let _user_id = get_user_id_from_request(
            &request.auth_token.unwrap(),
            context.synapse.clone(),
            context.users_cache.clone(),
        )
        .await;

        // Process rooom event as in process_room_event_ws()

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

async fn process_room_event_ws(
    event: FriendshipEventPayload,
    db: DatabaseComponent,
    user_id: String,
    token: Payload,
) {
    // Get current event
    let event = FriendshipEvent::ACCEPT;

    // Get user from event
    let acting_user = user_id;
    let second_user = "".to_string();

    // Get the friendship info
    let db_repos = &db.clone().db_repos.unwrap();
    let friendships_repository = &db_repos.friendships;
    let friendship = get_friendship(friendships_repository, &acting_user, &second_user)
        .await
        .unwrap();

    // Create room

    //  Get the last status from the database to later validate if the current action is valid.
    let friendship_history_repository = &db_repos.friendship_history;
    let _last_history = get_last_history(&friendship, friendship_history_repository)
        .await
        .unwrap();

    // Validate if the new status that is trying to be set is valid. If it's invalid or it has not changed, return here.
    let status = FriendshipStatus::Friends;

    // This is new: Get the Synapse room ID from our database.

    // This is new: Create a room if needed.

    // Start a database transaction.
    let friendship_ports = FriendshipPortsWs {
        db: &db,
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
    let room_info = RoomInfoWs {
        room_event: event,
        room_message_body: None,
        room_id: "",
    };
    let transaction = update_friendship_status(
        &friendship,
        "",
        "",
        status,
        room_info,
        friendship_ports,
        transaction,
    )
    .await;

    // If it's a friendship request event and the request contains a message, send a message event to the given room.

    // Store the friendship event in the given room.

    // End the database transaction.

    // Return the result.
}

async fn get_friendship(
    friendships_repository: &FriendshipsRepository,
    address_1: &str,
    address_2: &str,
) -> Result<Option<Friendship>, SynapseError> {
    let (friendship_result, _) = friendships_repository
        .get_friendship((address_1, address_2), None)
        .await;
    Ok(friendship_result.unwrap())
}

async fn get_last_history(
    friendship: &Option<Friendship>,
    friendship_history_repository: &FriendshipHistoryRepository,
) -> Result<Option<FriendshipHistory>, SynapseError> {
    let friendship = {
        match friendship {
            Some(friendship) => friendship,
            None => return Ok(None),
        }
    };

    let (friendship_history_result, _) = friendship_history_repository
        .get_last_history_for_friendship(friendship.id, None)
        .await;
    Ok(friendship_history_result.unwrap())
}

async fn update_friendship_status<'a>(
    friendship: &'a Option<Friendship>,
    acting_user: &'a str,
    second_user: &'a str,
    new_status: FriendshipStatus,
    room_info: RoomInfoWs<'a>,
    friendship_ports: FriendshipPortsWs<'a>,
    transaction: Transaction<'static, Postgres>,
) -> Result<Transaction<'static, Postgres>, SynapseError> {
    // store friendship update
    let is_active = new_status == FriendshipStatus::Friends;
    let (friendship_id_result, transaction) = store_friendship_update(
        friendship,
        is_active,
        acting_user,
        second_user,
        friendship_ports.friendships_repository,
        transaction,
    )
    .await;

    let friendship_id = match friendship_id_result {
        Ok(friendship_id) => friendship_id,
        Err(err) => {
            log::error!("Couldn't store friendship update {err}");
            let _ = transaction.rollback().await;

            todo!()
        }
    };
    let room_event = match serde_json::to_string(&room_info.room_event) {
        Ok(room_event_string) => room_event_string,
        Err(err) => {
            log::error!("Error serializing room event: {:?}", err);
            let _ = transaction.rollback().await;
            todo!();
        }
    };

    let metadata = room_info.room_message_body.map(|message| {
        sqlx::types::Json(FriendshipMetadata {
            message: Some(message.to_string()),
            synapse_room_id: Some(room_info.room_id.to_string()),
            migrated_from_synapse: None,
        })
    });

    // store history
    let (friendship_history_result, transaction) = friendship_ports
        .friendship_history_repository
        .create(
            friendship_id,
            &room_event,
            acting_user,
            metadata,
            Some(transaction),
        )
        .await;

    let transaction = transaction.unwrap();

    match friendship_history_result {
        Ok(_) => Ok(transaction),
        Err(err) => {
            log::error!("Couldn't store friendship history update: {:?}", err);
            let _ = transaction.rollback().await;
            todo!()
        }
    }
}

async fn store_friendship_update(
    friendship: &Option<Friendship>,
    is_active: bool,
    address_1: &str,
    address_2: &str,
    friendships_repository: &FriendshipsRepository,
    transaction: Transaction<'static, Postgres>,
) -> (Result<Uuid, SynapseError>, Transaction<'static, Postgres>) {
    match friendship {
        Some(friendship) => {
            let (res, transaction) = friendships_repository
                .update_friendship_status(&friendship.id, is_active, Some(transaction))
                .await;

            let res = match res {
                Ok(_) => Ok(friendship.id),
                Err(err) => {
                    log::warn!("Couldn't update friendship {err}");
                    todo!()
                }
            };

            (res, transaction.unwrap())
        }
        None => {
            let (friendship_id, transaction) = friendships_repository
                .create_new_friendships((address_1, address_2), false, Some(transaction))
                .await;
            (
                friendship_id.map_err(|err| {
                    log::warn!("Couldn't crate new friendship {err}");
                    todo!()
                }),
                transaction.unwrap(),
            )
        }
    }
}
