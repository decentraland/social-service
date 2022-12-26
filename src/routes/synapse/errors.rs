use actix_web::{HttpResponse, ResponseError};
use reqwest::StatusCode;
use thiserror::Error;

use crate::routes::v1::error::{CommonError, ErrorResponse};

#[derive(Error, Debug, PartialEq)]
pub enum SynapseError {
    #[error("")]
    CommonError(CommonError),
    #[error("Requested friendship was not found")]
    FriendshipNotFound,
}

impl SynapseError {
    pub fn name(&self) -> String {
        match self {
            Self::FriendshipNotFound => "FriendshipNotFound".to_string(),
            Self::CommonError(base) => base.name(),
        }
    }
}

impl ResponseError for SynapseError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::FriendshipNotFound => StatusCode::NOT_FOUND,
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
