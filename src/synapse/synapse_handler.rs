// Responsible for managing Synapse rooms and storing events in these rooms.
use std::collections::HashMap;

use crate::{
    components::synapse::{user_id_as_synapse_user_id, CreateRoomResponse, SynapseComponent},
    domain::{error::CommonError, friendship_event::FriendshipEvent},
    entities::friendships::Friendship,
};

/// Function used when creating a room and setting an alias,
/// when setting the alias as body of the POST request the alias must be ony the local part: — 'wombat', not '#wombat:example.com'
///
/// Builds a room alias name from a vector of user addresses by sorting them and joining them with a "+" separator.
///
/// * `acting_user` - The address of the acting user.
/// * `second_user` - The address of the second user.
fn build_room_local_alias(acting_user: &str, second_user: &str) -> String {
    let act_user_parsed = acting_user.to_ascii_lowercase();
    let sec_user_parsed: String = second_user.to_ascii_lowercase();

    let mut addresses = vec![act_user_parsed, sec_user_parsed];
    addresses.sort();

    addresses.join("+")
}

/// When interacting with a friend, then you need to join the room if you hadn't joined it yet.
/// Check out [here](https://spec.matrix.org/v1.3/client-server-api/#room-membership) to find out more about room membership and permissions based on the state in which a user may be.
pub async fn accept_room_invitation<'a>(
    token: &str,
    room_id: &str,
    synapse: &SynapseComponent,
) -> Result<(), CommonError> {
    let joined_rooms = synapse.get_joined_rooms(token).await;
    match joined_rooms {
        Ok(rooms) => {
            if !rooms.joined_rooms.contains(&room_id.to_string()) {
                // The room exists of a previous interaction between users, but the current user hasn't joined yet
                let joined_room = synapse.join_room(token, room_id).await;
                joined_room.map(|_| ())
            } else {
                Ok(())
            }
        }
        Err(err) => Err(err),
    }
}

/// Stores a message event in a Synapse room if it's a friendship request event and the request contains a message.
pub async fn store_message_in_synapse_room<'a>(
    token: &str,
    room_id: &str,
    room_event: FriendshipEvent,
    room_message_body: Option<&str>,
    synapse: &SynapseComponent,
) -> Result<(), CommonError> {
    // Check if it's a `request` event.
    if room_event != FriendshipEvent::REQUEST {
        return Ok(());
    }

    // Check if there is a message, if any, send the message event to the given room.
    if let Some(val) = room_message_body {
        // Check if the message body is not empty
        if !val.is_empty() {
            for retry_count in 0..3 {
                match synapse
                    .send_message_event_given_room(token, room_id, room_event, val)
                    .await
                {
                    Ok(_) => {
                        return Ok(());
                    }
                    Err(err) => {
                        if retry_count == 2 {
                            log::error!("[RPC] Store message in synapse room > Error {err}");
                            return Err(err);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Stores a room event in a Synapse room, and it returns the `EventResponse` containing the event id if the operation was successful
pub async fn store_room_event_in_synapse_room(
    token: &str,
    room_id: &str,
    room_event: FriendshipEvent,
    room_message_body: Option<&str>,
    synapse: &SynapseComponent,
) -> Result<(), CommonError> {
    let res = synapse
        .store_room_event(token, room_id, room_event, room_message_body)
        .await;

    match res {
        Ok(_) => Ok(()),
        Err(err) => {
            log::error!("[RPC] Store room event in synapse room > Error {err}");
            Err(err)
        }
    }
}

/// Creates a new private room in Synapse and returns the `CreateRoomResponse` if successful.
/// Returns a `FriendshipServiceError` if there is an error communicating with Synapse.
///
/// * `token` - A `&str` representing the auth token.
/// * `synapse_user_ids` - A `Vec<&str>` containing the user ids to invite to the room. There is no need to include the current user id.
/// * `room_alias_name` -
/// * `synapse` - A reference to the `SynapseComponent` instance.
async fn create_private_room_in_synapse(
    token: &str,
    synapse_user_ids: Vec<&str>,
    room_alias_name: String,
    synapse: &SynapseComponent,
) -> Result<CreateRoomResponse, CommonError> {
    let res = synapse
        .create_private_room(token, synapse_user_ids, &room_alias_name)
        .await;

    match res {
        Ok(response) => {
            let res = CreateRoomResponse {
                room_id: response.room_id,
            };
            Ok(res)
        }
        Err(err) => {
            log::error!("[RPC] Create private room in synapse > Error {err}");
            Err(err)
        }
    }
}

async fn get_room_id_for_alias_in_synapse(
    token: &str,
    room_alias_name: &str,
    synapse: &SynapseComponent,
) -> Result<String, CommonError> {
    let res = synapse
        .get_room_id_for_alias(token, room_alias_name, synapse)
        .await;

    match res {
        Ok(response) => Ok(response.room_id),
        Err(err) => {
            log::error!("[RPC] Get room id for alias in synapse > Error {err}");
            Err(err)
        }
    }
}

/// Creates a new Synapse room or returns the existing room id, depending on the `Friendship` and `FriendshipEvent`.
///
/// If the `Friendship` exists, returns the `synapse_room_id` in the `Friendship` struct.
///
/// If the `Friendship` does not exist and the `FriendshipEvent` is `REQUEST`, checks if a room with the alias exists in Synapse.
/// If the room exists, returns its id.
/// If the room does not exist, a new room is created and the new room id is returned.
///
/// If the `Friendship` does not exist and the `FriendshipEvent` is not `REQUEST`, a Client Error error is returned.
pub async fn get_or_create_synapse_room_id(
    friendship: Option<&Friendship>,
    new_event: &FriendshipEvent,
    acting_user: &str,
    second_user: &str,
    token: &str,
    synapse: &SynapseComponent,
) -> Result<String, CommonError> {
    match friendship {
        Some(friendship) => Ok(friendship.synapse_room_id.clone()),
        None => {
            if new_event == &FriendshipEvent::REQUEST {
                let room_alias_name: String = build_room_local_alias(acting_user, second_user);

                let get_room_result =
                    get_room_id_for_alias_in_synapse(token, &room_alias_name, synapse).await;

                match get_room_result {
                    Ok(room_id) => Ok(room_id),
                    Err(_) => {
                        let second_user_as_synapse_id =
                            user_id_as_synapse_user_id(second_user, &synapse.synapse_url);
                        let create_room_result = create_private_room_in_synapse(
                            token,
                            vec![&second_user_as_synapse_id],
                            room_alias_name,
                            synapse,
                        )
                        .await;

                        match create_room_result {
                            Ok(res) => Ok(res.room_id),
                            Err(err) => Err(err),
                        }
                    }
                }
            } else {
                log::error!("[RPC] Get or create synapse room > Friendship does not exists and the event is different than Request");
                Err(CommonError::BadRequest(
                    "Invalid frienship event update".to_owned(),
                ))
            }
        }
    }
}

/// Sets the account data event for the acting user.
///
/// Checkout the details [here](https://spec.matrix.org/v1.3/client-server-api/#mdirect).
/// Both the inviting client and the invitee’s client should record the fact that the room is a direct chat
/// by storing an m.direct event in the account data using /user/<user_id>/account_data/<type>.
///
/// Returns `Ok(())` if the account data was successfully set, or a `FriendshipServiceError` if an error occurs.
pub async fn set_account_data(
    token: &str,
    acting_user: &str,
    second_user: &str,
    room_id: &str,
    synapse: &SynapseComponent,
) -> Result<(), CommonError> {
    let acting_user_as_synapse_id = user_id_as_synapse_user_id(acting_user, &synapse.synapse_url);
    let m_direct_event = synapse
        .get_account_data(token, &acting_user_as_synapse_id)
        .await;

    let mut direct_room_map = match m_direct_event {
        Ok(m_direct_event) => {
            if !m_direct_event.direct.is_empty() {
                m_direct_event.direct.clone()
            } else {
                HashMap::new()
            }
        }
        Err(_) => HashMap::new(),
    };

    let second_user_as_synapse_id = user_id_as_synapse_user_id(second_user, &synapse.synapse_url);
    if let Some(room_ids) = direct_room_map.get_mut(&second_user_as_synapse_id) {
        if room_ids.contains(&room_id.to_string()) {
            return Ok(());
        } else {
            room_ids.push(room_id.to_string());
        }
    } else {
        direct_room_map.insert((&second_user).to_string(), vec![room_id.to_string()]);
    };

    synapse
        .set_account_data(token, &acting_user_as_synapse_id, direct_room_map)
        .await
        .map_err(|err| {
            log::error!("[RPC] Set account data > Error setting account data {err}");
            err
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::build_room_local_alias;

    #[test]
    fn build_room_alias_name_for_users() {
        let res = build_room_local_alias("0x1111ada11111", "0x1111ada11112");

        assert_eq!(res, "0x1111ada11111+0x1111ada11112");
    }
}
