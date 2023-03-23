use std::sync::Arc;

use dcl_rpc::stream_protocol::Generator;

use crate::{
    entities::friendships::FriendshipRepositoryImplementation, ws::app::MyContext, Empty,
    RequestEvents, ServerStreamResponse, SharedFriendshipsService,
    SubscribeFriendshipEventsUpdatesResponse, UpdateFriendshipPayload, UpdateFriendshipResponse,
    User, Users,
};

pub struct MyFriendshipsService {}

#[async_trait::async_trait]
impl SharedFriendshipsService<MyContext> for MyFriendshipsService {
    async fn get_friends(
        &self,
        _request: Empty,
        context: Arc<MyContext>,
    ) -> ServerStreamResponse<Users> {
        // Get user_id from somewhere in the ether
        let user_id = "";

        // Look for friendships and build friend addresses list
        let friendships = match &context.db.db_repos {
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

    async fn get_request_events(&self, _request: Empty, _context: Arc<MyContext>) -> RequestEvents {
        todo!()
    }

    async fn update_friendship_event(
        &self,
        _request: UpdateFriendshipPayload,
        _context: Arc<MyContext>,
    ) -> UpdateFriendshipResponse {
        todo!()
    }

    async fn subscribe_friendship_events_updates(
        &self,
        _request: Empty,
        _context: Arc<MyContext>,
    ) -> ServerStreamResponse<SubscribeFriendshipEventsUpdatesResponse> {
        todo!()
    }
}
