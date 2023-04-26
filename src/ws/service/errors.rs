use dcl_rpc::rpc_protocol::RemoteErrorResponse;
use thiserror::Error;

use crate::FriendshipErrors;

#[repr(i32)]
#[derive(Debug, PartialEq, Eq, Error)]
pub enum FriendshipsServiceError {
    #[error("Unknown: {0}")]
    Unknown(String) = FriendshipErrors::Unknown as i32,
    #[error("Bad request: {0}")]
    BadRequest(String) = FriendshipErrors::BadRequest as i32,
    #[error("Unauthorized: {0}")]
    Unauthorized(String) = FriendshipErrors::Unauthorized as i32,
    #[error("Forbidden: {0}")]
    Forbidden(String) = FriendshipErrors::Forbidden as i32,
    #[error("Not found")]
    NotFound = FriendshipErrors::NotFound as i32,
    #[error("Too many requests: {0}")]
    TooManyRequests(String) = FriendshipErrors::TooManyRequests as i32,
    #[error("Internal server error")]
    InternalServerError = FriendshipErrors::InternalServerError as i32,
}

impl RemoteErrorResponse for FriendshipsServiceError {
    fn error_code(&self) -> u32 {
        match self {
            Self::Unknown(_) => 0,
            Self::BadRequest(_) => 400,
            Self::Unauthorized(_) => 401,
            Self::Forbidden(_) => 403,
            Self::NotFound => 404,
            Self::TooManyRequests(_) => 429,
            Self::InternalServerError => 500,
        }
    }

    fn error_message(&self) -> String {
        match self {
            Self::Unknown(value) => format!("{self}: {value}"),
            Self::BadRequest(value) => format!("{self}: {value}"),
            Self::Unauthorized(value) => format!("{self}: {value}"),
            Self::Forbidden(value) => format!("{self}: {value}"),
            Self::NotFound => self.to_string(),
            Self::TooManyRequests(value) => format!("{self}: {value}"),
            Self::InternalServerError => self.to_string(),
        }
    }
}
