DROP INDEX IF EXISTS friendships_address_1;
DROP INDEX IF EXISTS friendships_address_2;

CREATE INDEX IF NOT EXISTS friendships_address_1_lower ON friendships (LOWER(address_1) text_pattern_ops);
CREATE INDEX IF NOT EXISTS friendships_address_2_lower ON friendships (LOWER(address_2) text_pattern_ops);
