use std::sync::Arc;

use crate::{
    ws::app::SocialContext, AuthToken, FriendshipsServiceServer, RequestEvents,
    ServerStreamResponse, SubscribeFriendshipEventsUpdatesResponse, UpdateFriendshipPayload,
    UpdateFriendshipResponse, User,
};

pub struct MyFriendshipsService {}

#[async_trait::async_trait]
impl FriendshipsServiceServer<SocialContext> for MyFriendshipsService {
    async fn get_friends(
        &self,
        _request: AuthToken,
        _context: Arc<SocialContext>,
    ) -> ServerStreamResponse<User> {
        todo!()
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
