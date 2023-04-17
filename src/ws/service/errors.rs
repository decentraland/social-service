use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug)]
pub struct FriendshipsServiceErrorResponse {
    pub code: u16,
    pub error: String,
    pub message: String,
}

#[derive(Error, Debug)]
pub enum FriendshipsServiceError {
    #[error("Internal Error")]
    InternalServerError,
    #[error("Unauthorized")]
    Unauthorized,
}

impl From<FriendshipsServiceError> for FriendshipsServiceErrorResponse {
    fn from(error: FriendshipsServiceError) -> Self {
        match error {
            FriendshipsServiceError::InternalServerError => FriendshipsServiceErrorResponse {
                code: 500,
                error: "Internal Error".to_string(),
                message: "An unknown internal error occurred".to_string(),
            },
            FriendshipsServiceError::Unauthorized => FriendshipsServiceErrorResponse {
                code: 401,
                error: "Unauthorized".to_string(),
                message: "Invalid or missing authentication token".to_string(),
            },
        }
    }
}
