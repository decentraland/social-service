use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: u16,
    pub error: String,
    pub message: String,
}

#[derive(Error, Debug, Clone)]
pub enum CommonError {
    #[error("Not found")]
    NotFound(String),
    #[error("Bad request {0}")]
    BadRequest(String),
    #[error("Requested user was not found")]
    UserNotFound(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("Unknown Internal Error")]
    Unknown(String),
    #[error("Unauthorized")]
    Unauthorized(String),
    #[error("Too many requests")]
    TooManyRequests(String),
}

impl PartialEq for CommonError {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

impl CommonError {
    pub fn name(&self) -> String {
        format!("{self:?}")
    }
}
