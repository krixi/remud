CREATE TABLE IF NOT EXISTS users
(
  id        INTEGER PRIMARY KEY NOT NULL,
  username  TEXT    UNIQUE      NOT NULL,
  password                      NOT NULL,
  salt                          NOT NULL
);