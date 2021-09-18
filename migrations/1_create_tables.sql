CREATE TABLE IF NOT EXISTS "config"
(
  key   TEXT UNIQUE NOT NULL,
  value TEXT        NOT NULL
);

CREATE TABLE IF NOT EXISTS "rooms"
(
  id          INTEGER PRIMARY KEY NOT NULL,
  description TEXT                NOT NULL
);

CREATE TABLE IF NOT EXISTS "objects"
(
  id          INTEGER PRIMARY KEY NOT NULL,
  keywords    TEXT                NOT NULL,
  name        TEXT                NOT NULL,
  description TEXT                NOT NULL,
  flags       INTEGER             NOT NULL 
);

CREATE TABLE IF NOT EXISTS "players"
(
  id        INTEGER PRIMARY KEY NOT NULL,
  username  TEXT    UNIQUE      NOT NULL,
  password                      NOT NULL,
  room      INTEGER             NOT NULL
);

CREATE TABLE IF NOT EXISTS "scripts"
(
  name    TEXT PRIMARY KEY NOT NULL,
  trigger TEXT             NOT NULL,
  code    TEXT             NOT NULL
);

CREATE TABLE IF NOT EXISTS "player_objects"
(
  player_id INTEGER        NOT NULL,
  object_id INTEGER UNIQUE NOT NULL,
  FOREIGN KEY (player_id)
    REFERENCES "players" (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE,
  FOREIGN KEY (object_id)
    REFERENCES "objects" (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "room_objects"
(
  room_id   INTEGER NOT NULL,
  object_id INTEGER UNIQUE NOT NULL,
  FOREIGN KEY (room_id)
    REFERENCES "rooms" (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE,
  FOREIGN KEY (object_id)
    REFERENCES "objects" (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "exits"
(
  room_from INTEGER NOT NULL,
  room_to   INTEGER NOT NULL,
  direction TEXT    NOT NULL,
  FOREIGN KEY (room_from)
    REFERENCES "rooms" (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE,
  FOREIGN KEY (room_to)
    REFERENCES "rooms" (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

DELETE FROM config;
INSERT INTO config (key, value)
VALUES
  ("spawn_room", "1");