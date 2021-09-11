ALTER TABLE player_inventories RENAME TO player_objects;

ALTER TABLE users RENAME TO players;

ALTER TABLE players ADD COLUMN room INTEGER NOT NULL DEFAULT 0;