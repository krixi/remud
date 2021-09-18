CREATE TABLE IF NOT EXISTS "player_scripts"
(
  player_id INTEGER NOT NULL,
  kind      TEXT    NOT NULL,
  script    TEXT    NOT NULL,
  trigger   TEXT    NOT NULL,
  FOREIGN KEY (player_id)
    REFERENCES "players" (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "object_scripts"
(
  object_id INTEGER NOT NULL,
  kind      TEXT    NOT NULL,
  script    TEXT    NOT NULL,
  trigger   TEXT    NOT NULL,
  FOREIGN KEY (object_id)
    REFERENCES "objects" (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "room_scripts"
(
  room_id INTEGER NOT NULL,
  kind    TEXT    NOT NULL,
  script  TEXT    NOT NULL,
  trigger TEXT    NOT NULL,
  FOREIGN KEY (room_id)
    REFERENCES "rooms" (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);