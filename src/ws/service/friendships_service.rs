use std::sync::Arc;

use dcl_rpc::stream_protocol::Generator;

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
                // Look for friendships and build friend addresses list
                let friendships = match context.app_components.db.db_repos.clone() {
                    Some(repos) => {
                        let (friendships, _) = repos
                            .friendships
                            .get_user_friends(&user_id.social_id, true, None)
                            .await;
                        match friendships {
                            Err(_) => todo!(),
                            Ok(it) => it,
                        }
                    }
                    None => todo!(),
                };

                let (generator, generator_yielder) = Generator::create();
                let mut users = Users::default();
                for friend in &friendships {
                    let user = User {
                        address: match friend.address_1.eq_ignore_ascii_case(&user_id.social_id) {
                            true => friend.address_2.to_string(),
                            false => friend.address_1.to_string(),
                        },
                    };
                    users.users.push(user);
                }
                generator_yielder.r#yield(users).await.unwrap();

                generator
            }
            Err(_er) => {
                let (g, _) = Generator::create();
                g
            }
        }
    }
    async fn get_request_events(&self, _context: Arc<SocialContext>) -> RequestEvents {
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
        _context: Arc<SocialContext>,
    ) -> ServerStreamResponse<SubscribeFriendshipEventsUpdatesResponse> {
        todo!()
    }
}
