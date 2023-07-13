// This query retrieves the intersecition of friends between two users
pub const MUTUALS_FRIENDS_QUERY: &str = "WITH friendsA AS (
  SELECT CASE
           WHEN LOWER(address_1) = LOWER($1) THEN address_2
           ELSE address_1
         END AS address
  FROM friendships
  WHERE LOWER(address_1) = LOWER($1) OR LOWER(address_2) = LOWER($1)
),
friendsB AS (
  SELECT CASE
           WHEN LOWER(address_1) = LOWER($2) THEN address_2
           ELSE address_1
         END AS address
  FROM friendships
  WHERE LOWER(address_1) = LOWER($2) OR LOWER(address_2) = LOWER($2)
)
SELECT address
FROM friendsA
WHERE address IN (SELECT address FROM friendsB);";

/// This query fetches the rows where the lastest event of a friendship_id is a REQUEST,
/// and either address_1 or address_2 is equal to the given user's address.
/// As there were tests cases that the request and accept event collided in timestamp, the guard to check if the friendship is inactive is needed
pub const USER_REQUESTS_QUERY: &str =
    "SELECT f.address_1, f.address_2, fh.acting_user, fh.timestamp, fh.metadata
      FROM friendships f
      INNER JOIN friendship_history fh ON f.id = fh.friendship_id
      WHERE (LOWER(f.address_1) = LOWER($1) OR LOWER(f.address_2) = LOWER($1))
      AND fh.event = '\"request\"'
      AND f.is_active IS FALSE
      AND fh.timestamp = (
        SELECT MAX(fh2.timestamp)
        FROM friendship_history fh2
        WHERE fh2.friendship_id = fh.friendship_id
      );";
