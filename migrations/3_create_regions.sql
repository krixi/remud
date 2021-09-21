CREATE TABLE IF NOT EXISTS "regions"
(
  id   INTEGER PRIMARY KEY NOT NULL,
  name TEXT    UNIQUE      NOT NULL
);

CREATE TABLE IF NOT EXISTS "room_regions"
(
  room_id   INTEGER NOT NULL,
  region_id INTEGER NOT NULL,
  UNIQUE(room_id, region_id),
  FOREIGN KEY (room_id)
    REFERENCES "rooms" (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE,
  FOREIGN KEY (region_id)
    REFERENCES "regions" (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);
