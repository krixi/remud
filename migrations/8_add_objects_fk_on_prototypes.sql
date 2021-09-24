
CREATE TABLE IF NOT EXISTS "new_objects"
(
  id              INTEGER PRIMARY KEY NOT NULL,
  prototype_id    INTEGER NOT NULL,
  inherit_scripts NOT NULL,
  name            TEXT,
  description     TEXT,
  flags           INTEGER,
  keywords        TEXT,
  FOREIGN KEY (prototype_id)
    REFERENCES "prototypes" (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

INSERT INTO "new_objects" (id, prototype_id, inherit_scripts, name, description, flags, keywords)
  SELECT id, prototype_id, inherit_scripts, name, description, flags, keywords FROM "objects";

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