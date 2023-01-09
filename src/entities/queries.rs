pub const MUTUALS_FRIENDS_QUERY: &str = "WITH friendsA as (
  SELECT
    CASE
      WHEN address_1 = $1 then address_2
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
          f_a.address_1 = $1
          or f_a.address_2 = $1
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
        WHEN address_1 = $2 then address_2
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
            f_b.address_1 = $2
            or f_b.address_2 = $2
          )
      ) as friends_b
  );";
