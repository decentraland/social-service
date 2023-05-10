use actix_web::{HttpResponse, ResponseError};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::friendships::{
    BadRequestError, ForbiddenError, InternalServerError, TooManyRequestsError, UnauthorizedError,
};

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: u16,
    pub error: String,
    pub message: String,
}

#[derive(Error, Debug)]
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

// Rest Error Responses mapping
impl ResponseError for CommonError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::UserNotFound(_) => StatusCode::NOT_FOUND,
            Self::TooManyRequests(_) => StatusCode::TOO_MANY_REQUESTS,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
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

pub enum WsServiceError {
    Unauthorized(UnauthorizedError),
    InternalServer(InternalServerError),
    BadRequest(BadRequestError),
    Forbidden(ForbiddenError),
    TooManyRequests(TooManyRequestsError),
}

pub fn as_ws_service(err: CommonError) -> WsServiceError {
    match err {
        CommonError::Forbidden(error_message) => WsServiceError::Forbidden(ForbiddenError {
            message: error_message,
        }),
        CommonError::Unauthorized(error_message) => {
            WsServiceError::Unauthorized(UnauthorizedError {
                message: error_message,
            })
        }
        CommonError::TooManyRequests(error_message) => {
            WsServiceError::TooManyRequests(TooManyRequestsError {
                message: error_message,
            })
        }
        CommonError::Unknown(error_message) => {
            WsServiceError::InternalServer(InternalServerError {
                message: error_message,
            })
        }
        CommonError::NotFound(error_message) => WsServiceError::BadRequest(BadRequestError {
            message: error_message,
        }),
        CommonError::BadRequest(error_message) => WsServiceError::BadRequest(BadRequestError {
            message: error_message,
        }),
        CommonError::UserNotFound(error_message) => WsServiceError::BadRequest(BadRequestError {
            message: error_message,
        }),
    }
}
