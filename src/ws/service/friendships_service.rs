use std::sync::Arc;

use crate::{Empty, MyExampleContext, SharedFriendshipsService, Users};

pub struct MyFriendshipsService {}

#[async_trait::async_trait]
impl SharedFriendshipsService<MyExampleContext> for MyFriendshipsService {
    async fn get_friends(&self, _request: Empty, _ctx: Arc<MyExampleContext>) -> Users {
        let users: Users = Users { users: [].to_vec() };

        users
    }
}
