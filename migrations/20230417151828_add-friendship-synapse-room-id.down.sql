-- Add down migration script here

ALTER TABLE friendships
DROP COLUMN synapse_room_id VARCHAR;

