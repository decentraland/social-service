DROP INDEX IF EXISTS friendships_address_1_lower;
DROP INDEX IF EXISTS friendships_address_2_lower;

CREATE INDEX IF NOT EXISTS friendships_address_1 ON friendships USING HASH (address_1);
CREATE INDEX IF NOT EXISTS friendships_address_2 ON friendships USING HASH (address_2);
