ALTER TABLE friendships
ADD COLUMN synapse_room_id VARCHAR;

UPDATE 
  friendships f
SET 
  synapse_room_id = metadata->>'synapse_room_id' 
FROM 
  friendship_history h
WHERE 
  f.id = h.friendship_id AND
  h.metadata IS NOT NULL AND
  NOT json_typeof(h.metadata) = 'null' AND
  h.metadata->>'synapse_room_id' IS NOT NULL AND
  NOT h.metadata->>'synapse_room_id' = 'null'
;
