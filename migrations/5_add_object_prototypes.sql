CREATE TABLE IF NOT EXISTS "prototypes"
(
  id          INTEGER PRIMARY KEY NOT NULL,
  keywords    TEXT                NOT NULL,
  name        TEXT                NOT NULL,
  description TEXT                NOT NULL,
  flags       INTEGER             NOT NULL 
);

CREATE TABLE IF NOT EXISTS "prototype_scripts"
(
  prototype_id INTEGER NOT NULL,
  kind         TEXT    NOT NULL,
  script       TEXT    NOT NULL,
  trigger      TEXT    NOT NULL,
  FOREIGN KEY (prototype_id)
    REFERENCES "prototypes" (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

INSERT INTO "prototypes" SELECT * from "objects";
INSERT INTO "prototype_scripts" SELECT * from "object_scripts";

CREATE TABLE IF NOT EXISTS "new_objects"
(
  id           INTEGER PRIMARY KEY NOT NULL,
  prototype_id INTEGER NOT NULL,
  keywords     TEXT,
  name         TEXT,
  description  TEXT,
  flags        INTEGER
);

INSERT INTO "new_objects" (id, prototype_id)
  SELECT objects.id, prototypes.id as prototype_id
  FROM "objects"
  INNER JOIN "prototypes" ON objects.name = prototypes.name;

CREATE TABLE IF NOT EXISTS "new_player_objects"
(
  player_id INTEGER        NOT NULL,
  object_id INTEGER UNIQUE NOT NULL,
  FOREIGN KEY (player_id)
    REFERENCES "players" (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE,
  FOREIGN KEY (object_id)
    REFERENCES "new_objects" (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

INSERT INTO "new_player_objects" SELECT * FROM "player_objects";

CREATE TABLE IF NOT EXISTS "new_room_objects"
(
  room_id   INTEGER NOT NULL,
  object_id INTEGER UNIQUE NOT NULL,
  FOREIGN KEY (room_id)
    REFERENCES "rooms" (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE,
  FOREIGN KEY (object_id)
    REFERENCES "new_objects" (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

INSERT INTO "new_room_objects" SELECT * FROM "room_objects";

DROP TABLE "objects";
DROP TABLE "room_objects";
DROP TABLE "player_objects";

ALTER TABLE "new_objects" RENAME TO "objects";
ALTER TABLE "new_room_objects" RENAME TO "room_objects";
ALTER TABLE "new_player_objects" RENAME TO "player_objects";