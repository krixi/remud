CREATE TABLE IF NOT EXISTS player_inventories
(
  player_id INTEGER        NOT NULL,
  object_id INTEGER UNIQUE NOT NULL,
  FOREIGN KEY (player_id)
    REFERENCES users (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE,
  FOREIGN KEY (object_id)
    REFERENCES objects (id)
      ON UPDATE NO ACTION
      ON DELETE CASCADE
);

-- Remove the useless salt column from the users table.
PRAGMA foreign_keys=off;

CREATE TABLE IF NOT EXISTS new_users
(
  id        INTEGER PRIMARY KEY NOT NULL,
  username  TEXT    UNIQUE      NOT NULL,
  password                      NOT NULL
);

INSERT INTO new_users (id, username, password)
  SELECT id, username, password
  FROM users;

DROP TABLE users;

ALTER TABLE new_users RENAME TO users;

PRAGMA foreign_keys=on;