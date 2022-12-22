#[derive(Debug)]
enum FriendshipEvent {
    REQUEST, // Send a friendship request
    CANCEL,  // Cancel a friendship request
    ACCEPT,  // Accept a friendship request
    REJECT,  // Reject a friendship request
    DELETE,  // Delete an existing friendship
}

impl FriendshipEvent {
    fn as_str(&self) -> String {
        format!("{self:?}").to_lowercase()
    }
}
