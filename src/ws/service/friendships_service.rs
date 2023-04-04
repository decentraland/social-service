use std::sync::Arc;

use dcl_rpc::stream_protocol::Generator;
use futures_util::StreamExt;

use crate::{
    entities::friendships::FriendshipRepositoryImplementation,
    ports::users_cache::get_user_id_from_token, ws::app::SocialContext, AuthToken,
    FriendshipsServiceServer, RequestEvents, ServerStreamResponse,
    SubscribeFriendshipEventsUpdatesResponse, UpdateFriendshipPayload, UpdateFriendshipResponse,
    User, Users,
};

pub struct MyFriendshipsService {}

#[async_trait::async_trait]
impl FriendshipsServiceServer<SocialContext> for MyFriendshipsService {
    async fn get_friends(
        &self,
        auth_token: AuthToken,
        context: Arc<SocialContext>,
    ) -> ServerStreamResponse<Users> {
        let user_id =
            get_user_id_from_token(context.app_components.clone(), &auth_token.synapse_token).await;

        match user_id {
            Ok(user_id) => {
                // Look for friendships
                let mut friendship = match context.app_components.db.db_repos.clone() {
                    Some(repos) => {
                        let friendship = repos
                            .friendships
                            .get_user_friends_stream(&user_id.social_id, true)
                            .await;
                        match friendship {
                            Err(_) => todo!(),
                            Ok(it) => it,
                        }
                    }
                    None => todo!(),
                };

                let (generator, generator_yielder) = Generator::create();

                tokio::spawn(async move {
                    let mut users = Users::default();

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
            Err(_er) => {
                // TODO: empty generator
                let (g, _) = Generator::create();
                g
            }
        }
    }
    async fn get_request_events(
        &self,
        _request: AuthToken,
        _context: Arc<SocialContext>,
    ) -> RequestEvents {
        todo!()
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
        _request: AuthToken,
        _context: Arc<SocialContext>,
    ) -> ServerStreamResponse<SubscribeFriendshipEventsUpdatesResponse> {
        todo!()
    }
}
