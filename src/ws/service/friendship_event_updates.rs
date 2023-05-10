use std::sync::Arc;

// TODO: Move this file to domain folder too

use crate::{
    components::database::DatabaseComponentImplementation,
    db::{
        friendships_handler::{get_friendship, get_last_history, update_friendship_status},
        types::FriendshipDbRepositories,
    },
    domain::{
        error::{as_ws_service, WsServiceError},
        room::RoomInfo,
    },
    domain::{
        friendship_event_validator::validate_new_event,
        friendship_status_calculator::get_new_friendship_status,
    },
    friendships::{InternalServerError, UnauthorizedError, UpdateFriendshipPayload},
    synapse::synapse_handler::{
        get_or_create_synapse_room_id, set_account_data, store_message_in_synapse_room,
        store_room_event_in_synapse_room,
    },
    ws::{app::SocialContext, service::types::EventResponse},
};

use super::mapper::events::update_request_as_event_payload;

/// Processes a friendship event update by validating it and updating the Database and Synapse.
pub async fn handle_friendship_update(
    request: UpdateFriendshipPayload,
    context: Arc<SocialContext>,
    acting_user: String,
) -> Result<EventResponse, WsServiceError> {
    let r = update_request_as_event_payload(request.clone());

    match r {
        Err(err) => Err(as_ws_service(err)),
        Ok(event_payload) => {
            let new_event = event_payload.friendship_event;
            let second_user = event_payload.second_user;

            let token = request
                .auth_token
                .as_ref()
                .ok_or_else(|| {
                    log::error!("Handle friendship update > `auth_token` is missing.");
                    WsServiceError::Unauthorized(UnauthorizedError {
                        message: "`auth_token` is missing".to_owned(),
                    })
                })?
                .synapse_token
                .as_ref()
                .ok_or_else(|| {
                    log::error!("Handle friendship update > `synapse_token` is missing.");
                    WsServiceError::Unauthorized(UnauthorizedError {
                        message: "`synapse_token` is missing".to_owned(),
                    })
                })?;

            let db_repos = context.db.clone().db_repos.ok_or_else(|| {
                log::error!("Handle friendship update > Db repositories > `repos` is None.");
                WsServiceError::InternalServer(InternalServerError {
                    message: "".to_owned(),
                })
            })?;

            // Get the friendship info
            let friendships_repository = &db_repos.friendships;
            let friendship = get_friendship(friendships_repository, &acting_user, &second_user)
                .await
                .or_else(|err| Err(as_ws_service(err)))?;

            let synapse_room_id = get_or_create_synapse_room_id(
                friendship.as_ref(),
                &new_event,
                &acting_user,
                &second_user,
                token,
                &context.synapse.clone(),
            )
            .await
            .or_else(|err| Err(as_ws_service(err)))?;

            if let Err(err) = set_account_data(
                token,
                &acting_user,
                &second_user,
                &synapse_room_id,
                &context.synapse,
            )
            .await
            {
                return Err(as_ws_service(err));
            }

            //  Get the last status from the database to later validate if the current action is valid.
            let friendship_history_repository = &db_repos.friendship_history;

            match get_last_history(friendship_history_repository, &friendship).await {
                Err(err) => {
                    return Err(as_ws_service(err));
                }
                Ok(last_recorded_history) => {
                    // Validate the new event is valid and different from the last recorded.
                    if let Err(err) = validate_new_event(&last_recorded_history, new_event) {
                        return Err(as_ws_service(err));
                    };

                    // Get new friendship status.
                    let n =
                        get_new_friendship_status(&acting_user, &last_recorded_history, new_event);
                    match n {
                        Err(err) => {
                            return Err(as_ws_service(err));
                        }
                        Ok(new_status) => {
                            // Start a database transaction.
                            let friendship_ports = FriendshipDbRepositories {
                                db: &context.db,
                                friendships_repository: &db_repos.friendships,
                                friendship_history_repository: &db_repos.friendship_history,
                            };
                            let transaction = match friendship_ports.db.start_transaction().await {
                                Ok(tx) => tx,
                                Err(error) => {
                                    log::error!("Handle friendship update > Couldn't start transaction to store friendship update {error}");
                                    return Err(WsServiceError::InternalServer(
                                        InternalServerError {
                                            message: "".to_owned(),
                                        },
                                    ));
                                }
                            };

                            // Update the friendship accordingly in the database. This means creating an entry in the friendships table or updating the is_active column.
                            let room_message_body =
                                event_payload.request_event_message_body.as_deref();
                            let room_info = RoomInfo {
                                room_event: new_event,
                                room_message_body,
                                room_id: synapse_room_id.as_str(),
                            };
                            let transaction = match update_friendship_status(
                                &friendship,
                                &acting_user,
                                &second_user,
                                new_status,
                                room_info,
                                friendship_ports,
                                transaction,
                            )
                            .await
                            {
                                Ok(tx) => tx,
                                Err(error) => {
                                    log::error!("Handle friendship update > Couldn't update friendship status {error}");
                                    return Err(as_ws_service(error));
                                }
                            };

                            // If it's a friendship request event and the request contains a message, send a message event to the given room.
                            if let Err(err) = store_message_in_synapse_room(
                                token,
                                synapse_room_id.as_str(),
                                new_event,
                                room_message_body,
                                &context.synapse,
                            )
                            .await
                            {
                                return Err(as_ws_service(err));
                            };

                            // Store the friendship event in the given room.
                            // We'll continue storing the event in Synapse to maintain the option to rollback to Matrix without losing any friendship interaction updates
                            let result = store_room_event_in_synapse_room(
                                token,
                                synapse_room_id.as_str(),
                                new_event,
                                room_message_body,
                                &context.synapse,
                            )
                            .await;

                            match result {
                                Ok(_) => {
                                    // End transaction
                                    let transaction_result = transaction.commit().await;

                                    match transaction_result {
                                        Ok(_) => Ok(EventResponse {
                                            user_id: second_user.to_string(),
                                        }),
                                        Err(err) => {
                                            log::error!("Handle friendship update > Couldn't end transaction to store friendship update {err}");
                                            Err(WsServiceError::InternalServer(
                                                InternalServerError {
                                                    message: "".to_owned(),
                                                },
                                            ))
                                        }
                                    }
                                }
                                Err(err) => Err(as_ws_service(err)),
                            }
                        }
                    }
                }
            }
        }
    }
}
