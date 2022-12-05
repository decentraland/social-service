CREATE TABLE IF NOT EXISTS user_features(
    "user" VARCHAR NOT NULL,
    feature_name VARCHAR NOT NULL,
    feature_value VARCHAR NOT NULL
);

CREATE UNIQUE INDEX idx_user_feature ON user_features ("user", feature_name);
