use actix_web::{HttpResponse, ResponseError};
use reqwest::StatusCode;
use serde::Serialize;
use thiserror::Error;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub code: u16,
    pub error: String,
    pub message: String,
}

#[derive(Error, Debug)]
pub enum CommonError {
    #[error("Requested user was not found")]
    UserNotFound,
    #[error("{0}")]
    Forbidden(String),
    #[error("Unknown Internal Error")]
    Unknown,
}

 impl CommonError {
    pub fn name(&self) -> String {
        match self {
            Self::UserNotFound => "UserNotFound".to_string(),
            Self::Forbidden(_str) => "Forbidden".to_string(),
            Self::Unknown => "Unknown".to_string(),
        }
    }
}
impl ResponseError for CommonError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UserNotFound => StatusCode::NOT_FOUND,
            Self::Forbidden(_str) => StatusCode::FORBIDDEN,
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

fn map_io_error(e: std::io::Error) -> CommonError {
    match e.kind() {
        std::io::ErrorKind::NotFound => CommonError::UserNotFound,
        std::io::ErrorKind::PermissionDenied => CommonError::Forbidden("".to_owned()),
        _ => CommonError::Unknown,
    }
}
