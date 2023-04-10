pub const MUTUALS_FRIENDS_QUERY: &str = "WITH friendsA as (
  SELECT
    CASE
      WHEN LOWER(address_1) = $1 then address_2
      else address_1
    end as address
  FROM
    (
      SELECT
        f_a.*
      from
        friendships f_a
      where
        (
          LOWER(f_a.address_1) = $1
          or LOWER(f_a.address_2) = $1
        )
    ) as friends_a
)
SELECT
  address
FROM
  friendsA f_b
WHERE
  address IN (
    SELECT
      CASE
        WHEN LOWER(address_1) = $2 then address_2
        else address_1
      end as address_a
    FROM
      (
        SELECT
          f_b.*
        from
          friendships f_b
        where
          (
            LOWER(f_b.address_1) = $2
            or LOWER(f_b.address_2) = $2
          )
      ) as friends_b
  );";

/// This query fetches the rows where the last event of a friendship_id is a REQUEST, and either address_1 or address_2 is equal to the given user's address.
pub const USER_REQUESTS_QUERY: &str = "SELECT f.address_1, f.address_2, 
fh.acting_user, fh.timestamp, fh.metadata
FROM friendship_history fh
  JOIN (
    SELECT friendship_id, MAX(timestamp) AS last_timestamp
      FROM friendship_history
      WHERE event = 'REQUEST'
      GROUP BY friendship_id
  ) h ON fh.friendship_id = h.friendship_id 
    AND fh.timestamp = h.last_timestamp
  JOIN friendships f ON f.id = fh.friendship_id 
    AND (LOWER(f.address_1) = $1 
    OR LOWER(f.address_2) = $1);";
