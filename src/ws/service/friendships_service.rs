use std::sync::Arc;

use crate::{
    ws::app::MyExampleContext, FriendshipsServiceServer, RequestEvents, ServerStreamResponse,
    SubscribeFriendshipEventsUpdatesResponse, UpdateFriendshipPayload, UpdateFriendshipResponse,
    Users,
};

pub struct MyFriendshipsService {}

#[async_trait::async_trait]
impl FriendshipsServiceServer<MyExampleContext> for MyFriendshipsService {
    async fn get_friends(&self, _context: Arc<MyExampleContext>) -> ServerStreamResponse<Users> {
        todo!()
    }
    async fn get_request_events(&self, _context: Arc<MyExampleContext>) -> RequestEvents {
        todo!()
    }

    async fn update_friendship_event(
        &self,
        _request: UpdateFriendshipPayload,
        _context: Arc<MyExampleContext>,
    ) -> UpdateFriendshipResponse {
        todo!()
    }

    async fn subscribe_friendship_events_updates(
        &self,
        _context: Arc<MyExampleContext>,
    ) -> ServerStreamResponse<SubscribeFriendshipEventsUpdatesResponse> {
        todo!()
    }
}
