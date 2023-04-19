/// Builds a room alias name from a vector of user addresses by sorting them and joining them with a "+" separator.
///
/// * `user_ids` - A mut vector of users addresses as strings.
///
/// Returns the room alias name as a string.
pub fn build_room_alias_name(mut user_ids: Vec<&str>) -> String {
    user_ids.sort();
    user_ids.join("+")
}
