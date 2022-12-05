CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE TABLE IF NOT EXISTS friendships(
    id uuid DEFAULT uuid_generate_v4(),
    address_1 VARCHAR NOT NULL,
    address_2 VARCHAR NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (id)
);
