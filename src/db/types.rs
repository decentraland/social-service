use crate::{
    components::database::DatabaseComponent,
    entities::{
        friendship_history::FriendshipHistoryRepository, friendships::FriendshipsRepository,
    },
};

pub struct FriendshipDbRepositories<'a> {
    pub db: &'a DatabaseComponent,
    pub friendships_repository: &'a FriendshipsRepository,
    pub friendship_history_repository: &'a FriendshipHistoryRepository,
}
