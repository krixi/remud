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