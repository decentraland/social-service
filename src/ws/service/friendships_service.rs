use std::sync::Arc;

use dcl_rpc::stream_protocol::Generator;
use futures_util::StreamExt;

use crate::{
    entities::friendships::FriendshipRepositoryImplementation,
    ports::users_cache::get_user_id_from_token, ws::app::SocialContext, FriendshipsServiceServer,
    Payload, RequestEvents, ServerStreamResponse, SubscribeFriendshipEventsUpdatesResponse,
    UpdateFriendshipPayload, UpdateFriendshipResponse, User, Users,
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
        // Get user id from the auth token
        let user_id = match request.synapse_token {
            Some(token) => get_user_id_from_token(context.app_components.clone(), &token).await,
            None => {
                // TODO: Handle no auth token.
                log::error!("Get Friends > Get User ID from Token > `synapse_token` is None.");
                // Err(FriendshipsError::CommonError(CommonError::Unauthorized)),
                todo!()
            }
        };

        match user_id {
            Ok(user_id) => {
                let social_id = user_id.social_id.clone();
                log::info!("Getting all friends for user: {}", social_id);
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
                                log::error!(
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
            Err(err) => {
                // TODO: Handle error when trying to get User Id.
                log::error!("Get Friends > Get User ID from Token > Error: {err}.");
                // Err(FriendshipsError::CommonError(CommonError::Unknown)),
                let (g, _) = Generator::create();
                g
            }
        }
    }
    #[tracing::instrument(name = "RPC SERVER > Get Request Events", skip(_request, _context))]
    async fn get_request_events(
        &self,
        _request: Payload,
        _context: Arc<SocialContext>,
    ) -> RequestEvents {
        todo!()
    }

    #[tracing::instrument(
        name = "RPC SERVER > Update Friendship Event",
        skip(_request, _context)
    )]
    async fn update_friendship_event(
        &self,
        _request: UpdateFriendshipPayload,
        _context: Arc<SocialContext>,
    ) -> UpdateFriendshipResponse {
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
