use std::sync::Arc;

use crate::{
    ws::app::MyExampleContext, Empty, RequestEvents, ServerStreamResponse,
    SharedFriendshipsService, SubscribeFriendshipEventsUpdatesResponse, UpdateFriendshipPayload,
    UpdateFriendshipResponse, Users,
};

pub struct MyFriendshipsService {}

#[async_trait::async_trait]
impl SharedFriendshipsService<MyExampleContext> for MyFriendshipsService {
    async fn get_friends(
        &self,
        _request: Empty,
        _context: Arc<MyExampleContext>,
    ) -> ServerStreamResponse<Users> {
        todo!()
    }
    async fn get_request_events(
        &self,
        _request: Empty,
        _context: Arc<MyExampleContext>,
    ) -> RequestEvents {
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
        _request: Empty,
        _context: Arc<MyExampleContext>,
    ) -> ServerStreamResponse<SubscribeFriendshipEventsUpdatesResponse> {
        todo!()
    }
}
