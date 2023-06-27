use crate::{
    domain::error::CommonError,
    friendships::{
        request_events_response, subscribe_friendship_events_updates_response,
        update_friendship_response, users_response, BadRequestError, ForbiddenError,
        InternalServerError, RequestEventsResponse, SubscribeFriendshipEventsUpdatesResponse,
        TooManyRequestsError, UnauthorizedError, UpdateFriendshipResponse, UsersResponse,
    },
};

#[derive(Clone)]
pub enum WsServiceError {
    Unauthorized(UnauthorizedError),
    InternalServer(InternalServerError),
    BadRequest(BadRequestError),
    Forbidden(ForbiddenError),
    TooManyRequests(TooManyRequestsError),
}

impl From<UnauthorizedError> for WsServiceError {
    fn from(value: UnauthorizedError) -> Self {
        WsServiceError::Unauthorized(value)
    }
}
impl From<InternalServerError> for WsServiceError {
    fn from(value: InternalServerError) -> Self {
        WsServiceError::InternalServer(value)
    }
}
impl From<BadRequestError> for WsServiceError {
    fn from(value: BadRequestError) -> Self {
        WsServiceError::BadRequest(value)
    }
}
impl From<ForbiddenError> for WsServiceError {
    fn from(value: ForbiddenError) -> Self {
        WsServiceError::Forbidden(value)
    }
}
impl From<TooManyRequestsError> for WsServiceError {
    fn from(value: TooManyRequestsError) -> Self {
        WsServiceError::TooManyRequests(value)
    }
}

impl From<CommonError> for WsServiceError {
    fn from(value: CommonError) -> Self {
        match value {
            CommonError::NotFound(message) => {
                WsServiceError::BadRequest(BadRequestError { message })
            }
            CommonError::BadRequest(message) => {
                WsServiceError::BadRequest(BadRequestError { message })
            }
            CommonError::UserNotFound(message) => {
                WsServiceError::BadRequest(BadRequestError { message })
            }
            CommonError::Forbidden(message) => {
                WsServiceError::Forbidden(ForbiddenError { message })
            }
            CommonError::Unknown(message) => {
                WsServiceError::InternalServer(InternalServerError { message })
            }
            CommonError::Unauthorized(message) => {
                WsServiceError::Unauthorized(UnauthorizedError { message })
            }
            CommonError::TooManyRequests(message) => {
                WsServiceError::TooManyRequests(TooManyRequestsError { message })
            }
        }
    }
}

impl From<CommonError> for UsersResponse {
    fn from(value: CommonError) -> Self {
        let err: WsServiceError = value.into();
        match err {
            WsServiceError::Unauthorized(err) => {
                UsersResponse::from_response(users_response::Response::UnauthorizedError(err))
            }
            WsServiceError::InternalServer(err) => {
                UsersResponse::from_response(users_response::Response::InternalServerError(err))
            }
            WsServiceError::Forbidden(err) => {
                UsersResponse::from_response(users_response::Response::ForbiddenError(err))
            }
            WsServiceError::TooManyRequests(err) => {
                UsersResponse::from_response(users_response::Response::TooManyRequestsError(err))
            }
            WsServiceError::BadRequest(err) => UsersResponse::from_response(
                users_response::Response::InternalServerError(InternalServerError {
                    message: err.message,
                }),
            ),
        }
    }
}

impl From<CommonError> for RequestEventsResponse {
    fn from(value: CommonError) -> Self {
        let err: WsServiceError = value.into();
        match err {
            WsServiceError::Unauthorized(err) => RequestEventsResponse::from_response(
                request_events_response::Response::UnauthorizedError(err),
            ),
            WsServiceError::InternalServer(err) => RequestEventsResponse::from_response(
                request_events_response::Response::InternalServerError(err),
            ),
            WsServiceError::Forbidden(err) => RequestEventsResponse::from_response(
                request_events_response::Response::ForbiddenError(err),
            ),
            WsServiceError::TooManyRequests(err) => RequestEventsResponse::from_response(
                request_events_response::Response::TooManyRequestsError(err),
            ),
            WsServiceError::BadRequest(err) => RequestEventsResponse::from_response(
                request_events_response::Response::InternalServerError(InternalServerError {
                    message: err.message,
                }),
            ),
        }
    }
}

impl From<CommonError> for UpdateFriendshipResponse {
    fn from(value: CommonError) -> Self {
        let err: WsServiceError = value.into();
        match err {
            WsServiceError::Unauthorized(err) => UpdateFriendshipResponse::from_response(
                update_friendship_response::Response::UnauthorizedError(err),
            ),
            WsServiceError::InternalServer(err) => UpdateFriendshipResponse::from_response(
                update_friendship_response::Response::InternalServerError(err),
            ),
            WsServiceError::BadRequest(err) => UpdateFriendshipResponse::from_response(
                update_friendship_response::Response::BadRequestError(err),
            ),
            WsServiceError::Forbidden(err) => UpdateFriendshipResponse::from_response(
                update_friendship_response::Response::ForbiddenError(err),
            ),
            WsServiceError::TooManyRequests(err) => UpdateFriendshipResponse::from_response(
                update_friendship_response::Response::TooManyRequestsError(err),
            ),
        }
    }
}

impl From<CommonError> for SubscribeFriendshipEventsUpdatesResponse {
    fn from(value: CommonError) -> Self {
        let err: WsServiceError = value.into();
        match err {
            WsServiceError::Unauthorized(err) => {
                SubscribeFriendshipEventsUpdatesResponse::from_response(
                    subscribe_friendship_events_updates_response::Response::UnauthorizedError(err),
                )
            }
            WsServiceError::InternalServer(err) => {
                SubscribeFriendshipEventsUpdatesResponse::from_response(
                    subscribe_friendship_events_updates_response::Response::InternalServerError(
                        err,
                    ),
                )
            }
            WsServiceError::Forbidden(err) => {
                SubscribeFriendshipEventsUpdatesResponse::from_response(
                    subscribe_friendship_events_updates_response::Response::ForbiddenError(err),
                )
            }
            WsServiceError::TooManyRequests(err) => {
                SubscribeFriendshipEventsUpdatesResponse::from_response(
                    subscribe_friendship_events_updates_response::Response::TooManyRequestsError(
                        err,
                    ),
                )
            }
            WsServiceError::BadRequest(err) => {
                SubscribeFriendshipEventsUpdatesResponse::from_response(
                    subscribe_friendship_events_updates_response::Response::InternalServerError(
                        InternalServerError {
                            message: err.message,
                        },
                    ),
                )
            }
        }
    }
}
