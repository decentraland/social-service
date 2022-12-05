CREATE TABLE IF NOT EXISTS friendship_history(
    id uuid DEFAULT uuid_generate_v4(),
    friendship_id uuid NOT NULL,
    event VARCHAR NOT NULL,
    acting_user VARCHAR NOT NULL,
    metadata json NULL,
    timestamp timestamp DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (id)
);