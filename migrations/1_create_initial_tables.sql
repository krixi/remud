CREATE TABLE IF NOT EXISTS rooms
(
  id          INTEGER PRIMARY KEY NOT NULL,
  description TEXT                NOT NULL
);

INSERT INTO rooms (id, description)
VALUES
  (1, "A dull white light permeates this shapeless space.");

CREATE TABLE IF NOT EXISTS config
(
  key   TEXT UNIQUE NOT NULL,
  value TEXT        NOT NULL
);

INSERT INTO config (key, value)
VALUES
  ("spawn_room", "1");

CREATE TABLE IF NOT EXISTS exits
(
  room_from INTEGER NOT NULL,
  room_to   INTEGER NOT NULL,
  direction TEXT    NOT NULL,
  FOREIGN KEY (room_from)
    REFERENCES rooms (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE,
  FOREIGN KEY (room_to)
    REFERENCES rooms (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);