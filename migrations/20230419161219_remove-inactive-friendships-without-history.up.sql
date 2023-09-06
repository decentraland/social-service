DELETE FROM friendships WHERE id IN(
  SELECT f.id 
	FROM friendships f
	LEFT JOIN friendship_history h
		ON f.id = h.friendship_id
	WHERE
		h.friendship_id IS NULL AND
		f.id IS NOT NULL AND
		f.synapse_room_id IS NULL AND
		f.is_active = false
);
