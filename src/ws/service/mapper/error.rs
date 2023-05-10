use crate::{
    domain::error::CommonError,
    friendships::{
        request_events_response, subscribe_friendship_events_updates_response,
        update_friendship_response, users_response, BadRequestError, ForbiddenError,
        InternalServerError, RequestEventsResponse, SubscribeFriendshipEventsUpdatesResponse,
        TooManyRequestsError, UnauthorizedError, UpdateFriendshipResponse, UsersResponse,
    },
};

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

pub fn to_user_response(err: CommonError) -> UsersResponse {
    let err = as_ws_service(err);
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

pub fn to_request_events_response(err: CommonError) -> RequestEventsResponse {
    let err = as_ws_service(err);
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

pub fn to_update_friendship_response(err: CommonError) -> UpdateFriendshipResponse {
    let err = as_ws_service(err);
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

pub fn to_subscribe_friendship_events_updates_response(
    err: CommonError,
) -> SubscribeFriendshipEventsUpdatesResponse {
    let err = as_ws_service(err);
    match err {
        WsServiceError::Unauthorized(err) => {
            SubscribeFriendshipEventsUpdatesResponse::from_response(
                subscribe_friendship_events_updates_response::Response::UnauthorizedError(err),
            )
        }
        WsServiceError::InternalServer(err) => {
            SubscribeFriendshipEventsUpdatesResponse::from_response(
                subscribe_friendship_events_updates_response::Response::InternalServerError(err),
            )
        }
        WsServiceError::Forbidden(err) => SubscribeFriendshipEventsUpdatesResponse::from_response(
            subscribe_friendship_events_updates_response::Response::ForbiddenError(err),
        ),
        WsServiceError::TooManyRequests(err) => {
            SubscribeFriendshipEventsUpdatesResponse::from_response(
                subscribe_friendship_events_updates_response::Response::TooManyRequestsError(err),
            )
        }
        WsServiceError::BadRequest(err) => SubscribeFriendshipEventsUpdatesResponse::from_response(
            subscribe_friendship_events_updates_response::Response::InternalServerError(
                InternalServerError {
                    message: err.message,
                },
            ),
        ),
    }
}
