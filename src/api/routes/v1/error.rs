use actix_web::{HttpResponse, ResponseError};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: u16,
    pub error: String,
    pub message: String,
}

#[derive(Error, Debug)]
pub enum CommonError {
    #[error("Not found")]
    NotFound,
    #[error("Bad request {0}")]
    BadRequest(String),
    #[error("Requested user was not found")]
    UserNotFound,
    #[error("{0}")]
    Forbidden(String),
    #[error("Unknown Internal Error")]
    Unknown,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Too many requests")]
    TooManyRequests,
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

impl ResponseError for CommonError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::UserNotFound => StatusCode::NOT_FOUND,
            Self::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
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
