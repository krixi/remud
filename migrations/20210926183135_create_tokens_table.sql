CREATE TABLE IF NOT EXISTS 'tokens'
(
  player_id   INTEGER PRIMARY KEY NOT NULL,
  access      INTEGER             NOT NULL,
  refresh     INTEGER             NOT NULL
);
