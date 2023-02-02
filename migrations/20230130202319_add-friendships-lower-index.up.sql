DROP INDEX IF EXISTS friendships_address_1;
DROP INDEX IF EXISTS friendships_address_2;

CREATE INDEX IF NOT EXISTS friendships_address_1_lower ON friendships (LOWER(address_1) text_pattern_ops);
CREATE INDEX IF NOT EXISTS friendships_address_2_lower ON friendships (LOWER(address_2) text_pattern_ops);

ALTER TABLE friendships
  ADD CONSTRAINT unique_addresses UNIQUE(address_1, address_2);

ALTER TABLE friendships
ADD CONSTRAINT address_1_smaller_than_address_2 CHECK (address_1 < address_2);

