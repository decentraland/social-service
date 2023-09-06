
UPDATE
    friendship_history
  SET
    metadata = '{"migrated_from_synapse":  true}',
    timestamp = CURRENT_TIMESTAMP
WHERE 
    (event = 'accept' OR event = '"accept"')
  AND
    timestamp < timestamp '2023-02-06 16:55:16.208386'
  AND
    friendship_id in (
      SELECT
        colision.friendship_id
      FROM
        (
          SELECT fh.friendship_id, count(1) as total
          FROM friendships f
          INNER JOIN friendship_history fh ON f.id = fh.friendship_id
          WHERE 
            f.is_active IS true AND
            fh.timestamp = (
              SELECT MAX(fh2.timestamp)
              FROM friendship_history fh2
              WHERE fh2.friendship_id = fh.friendship_id
            )
          GROUP BY fh.friendship_id 
          order by total DESC
        )
        AS colision
      WHERE total > 1
    );
