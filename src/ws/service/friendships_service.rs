use std::sync::Arc;

use dcl_rpc::stream_protocol::Generator;

use crate::{
    entities::friendships::FriendshipRepositoryImplementation, ws::app::SocialContext, AuthToken,
    FriendshipsServiceServer, RequestEvents, ServerStreamResponse,
    SubscribeFriendshipEventsUpdatesResponse, UpdateFriendshipPayload, UpdateFriendshipResponse,
    User, Users,
};

pub struct MyFriendshipsService {}

#[async_trait::async_trait]
impl FriendshipsServiceServer<SocialContext> for MyFriendshipsService {
    async fn get_friends(
        &self,
        _synapse_token: AuthToken,
        context: Arc<SocialContext>,
    ) -> ServerStreamResponse<Users> {
        // Get user_id from somewhere in the ether
        let user_id = "0x1E205073F466D1544133B18Ad3f5634C4086A4d1";

        let cxt = &*context.clone();

        // Look for friendships and build friend addresses list
        let friendships = match cxt.app_components.db.db_repos.clone() {
            Some(repos) => {
                let (friendships, _) = repos
                    .friendships
                    .get_user_friends(user_id, true, None)
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
                address: friend.address_1.clone(),
            };
            users.users.push(user);
        }
        generator_yielder.r#yield(users).await.unwrap();

        generator
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
