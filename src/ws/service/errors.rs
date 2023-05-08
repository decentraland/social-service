use crate::friendships::FriendshipServiceError;
#[derive(Debug)]
#[repr(i32)]
pub enum DomainErrorCode {
    Unknown = 0,
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    TooManyRequests = 429,
    InternalServerError = 500,
}

pub fn as_service_error(code: DomainErrorCode, message: &str) -> FriendshipServiceError {
    let message = format!("{:?}: {}", code, message);
    FriendshipServiceError {
        code: code as i32,
        message,
    }
}

#[cfg(test)]
mod test {
    use super::{as_service_error, DomainErrorCode};

    #[test]
    fn test_error_code() {
        let service_error =
            as_service_error(DomainErrorCode::NotFound, &"user not found".to_string());

        assert_eq!(service_error.code, 404);
        assert_eq!(service_error.message, "NotFound: user not found");
    }
}
