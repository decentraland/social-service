use std::sync::Arc;

use crate::{
    ws::app::SocialContext, FriendshipsServiceServer, RequestEvents, ServerStreamResponse,
    SubscribeFriendshipEventsUpdatesResponse, UpdateFriendshipPayload, UpdateFriendshipResponse,
    Users,
};

pub struct MyFriendshipsService {}

#[async_trait::async_trait]
impl FriendshipsServiceServer<SocialContext> for MyFriendshipsService {
    async fn get_friends(&self, _context: Arc<SocialContext>) -> ServerStreamResponse<Users> {
        todo!()
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
