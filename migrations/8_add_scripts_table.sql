CREATE TABLE IF NOT EXISTS scripts
(
  name    TEXT PRIMARY KEY NOT NULL,
  trigger TEXT             NOT NULL,
  code    TEXT             NOT NULL
);

-- Update objects column names
PRAGMA foreign_keys=off;

CREATE TABLE IF NOT EXISTS new_objects
(
  id          INTEGER PRIMARY KEY NOT NULL,
  keywords    TEXT                NOT NULL,
  name        TEXT                NOT NULL,
  description TEXT                NOT NULL
);

INSERT INTO new_objects (id, keywords, name, description)
  SELECT id, keywords, short AS name, long AS description
  FROM objects;

DROP TABLE objects;

ALTER TABLE new_objects RENAME TO objects;

PRAGMA foreign_keys=on;