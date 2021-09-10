CREATE TABLE IF NOT EXISTS objects
(
  id       INTEGER PRIMARY KEY NOT NULL,
  keywords TEXT                NOT NULL,
  short    TEXT                NOT NULL,
  long     TEXT                NOT NULL
);

CREATE TABLE IF NOT EXISTS room_objects
(
  room_id   INTEGER NOT NULL,
  object_id INTEGER UNIQUE NOT NULL,
  FOREIGN KEY (room_id)
    REFERENCES rooms (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE,
  FOREIGN KEY (object_id)
    REFERENCES objects (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);