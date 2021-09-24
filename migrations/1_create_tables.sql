CREATE TABLE IF NOT EXISTS 'config'
(
  key   TEXT UNIQUE NOT NULL,
  value TEXT        NOT NULL
);

CREATE TABLE IF NOT EXISTS 'players'
(
  id          INTEGER PRIMARY KEY NOT NULL,
  username    TEXT    UNIQUE      NOT NULL,
  password                        NOT NULL,
  description TEXT                NOT NULL,
  flags       INTEGER             NOT NULL,
  room        INTEGER             NOT NULL
);

CREATE TABLE IF NOT EXISTS 'rooms'
(
  id          INTEGER PRIMARY KEY NOT NULL,
  name        TEXT                NOT NULL,
  description TEXT                NOT NULL
);

CREATE TABLE IF NOT EXISTS 'exits'
(
  room_from INTEGER NOT NULL,
  room_to   INTEGER NOT NULL,
  direction TEXT    NOT NULL,
  FOREIGN KEY (room_from)
    REFERENCES 'rooms' (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE,
  FOREIGN KEY (room_to)
    REFERENCES 'rooms' (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS 'prototypes'
(
  id          INTEGER PRIMARY KEY NOT NULL,
  name        TEXT                NOT NULL,
  description TEXT                NOT NULL,
  flags       INTEGER             NOT NULL,
  keywords    TEXT                NOT NULL
);

CREATE TABLE IF NOT EXISTS 'objects'
(
  id              INTEGER PRIMARY KEY NOT NULL,
  prototype_id    INTEGER             NOT NULL,
  inherit_scripts                     NOT NULL,
  name            TEXT                NOT NULL,
  description     TEXT                NOT NULL,
  flags           INTEGER             NOT NULL,
  keywords        TEXT                NOT NULL,
  FOREIGN KEY (prototype_id)
    REFERENCES 'prototypes' (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS 'player_objects'
(
  player_id INTEGER        NOT NULL,
  object_id INTEGER UNIQUE NOT NULL,
  FOREIGN KEY (player_id)
    REFERENCES 'players' (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE,
  FOREIGN KEY (object_id)
    REFERENCES 'objects' (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS 'room_objects'
(
  room_id   INTEGER NOT NULL,
  object_id INTEGER UNIQUE NOT NULL,
  FOREIGN KEY (room_id)
    REFERENCES 'rooms' (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE,
  FOREIGN KEY (object_id)
    REFERENCES 'objects' (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS 'scripts'
(
  name    TEXT PRIMARY KEY NOT NULL,
  trigger TEXT             NOT NULL,
  code    TEXT             NOT NULL
);

CREATE TABLE IF NOT EXISTS 'player_scripts'
(
  player_id INTEGER NOT NULL,
  kind      TEXT    NOT NULL,
  script    TEXT    NOT NULL,
  trigger   TEXT    NOT NULL,
  FOREIGN KEY (player_id)
    REFERENCES 'players' (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS 'room_scripts'
(
  room_id INTEGER NOT NULL,
  kind    TEXT    NOT NULL,
  script  TEXT    NOT NULL,
  trigger TEXT    NOT NULL,
  FOREIGN KEY (room_id)
    REFERENCES 'rooms' (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS 'prototype_scripts'
(
  prototype_id INTEGER NOT NULL,
  kind         TEXT    NOT NULL,
  script       TEXT    NOT NULL,
  trigger      TEXT    NOT NULL,
  FOREIGN KEY (prototype_id)
    REFERENCES 'prototypes' (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS 'object_scripts'
(
  object_id INTEGER NOT NULL,
  kind      TEXT    NOT NULL,
  script    TEXT    NOT NULL,
  trigger   TEXT    NOT NULL,
  FOREIGN KEY (object_id)
    REFERENCES 'objects' (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS 'regions'
(
  id   INTEGER PRIMARY KEY NOT NULL,
  name TEXT    UNIQUE      NOT NULL
);

CREATE TABLE IF NOT EXISTS 'room_regions'
(
  room_id   INTEGER NOT NULL,
  region_id INTEGER NOT NULL,
  UNIQUE(room_id, region_id),
  FOREIGN KEY (room_id)
    REFERENCES 'rooms' (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE,
  FOREIGN KEY (region_id)
    REFERENCES 'regions' (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

DELETE FROM config;
INSERT INTO config (key, value)
VALUES
  ('spawn_room', '1');