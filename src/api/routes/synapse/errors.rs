use actix_web::{HttpResponse, ResponseError};
use reqwest::StatusCode;
use thiserror::Error;

use crate::domain::error::{CommonError, ErrorResponse};

#[derive(Error, Debug, PartialEq)]
pub enum SynapseError {
    #[error("")]
    CommonError(CommonError),
    #[error("Requested friendship was not found")]
    FriendshipNotFound,
    #[error("The sent friendship interaction is not valid at this moment")]
    InvalidEvent,
}

impl SynapseError {
    pub fn name(&self) -> String {
        match self {
            Self::FriendshipNotFound => "FriendshipNotFound".to_string(),
            Self::InvalidEvent => "InvalidEvent".to_string(),
            Self::CommonError(base) => base.name(),
        }
    }
}

impl ResponseError for SynapseError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::FriendshipNotFound => StatusCode::NOT_FOUND,
            Self::InvalidEvent => StatusCode::BAD_REQUEST,
            Self::CommonError(base) => base.status_code(),
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let error_response = ErrorResponse {
            code: status_code.as_u16(),
            message: self.to_string(),
            error: self.name(),
        };
        HttpResponse::build(status_code).json(error_response)
    }
}
