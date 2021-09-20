- add mechanism to automatically clear colors at the end of the message if they were set

- add mechanism for immortal info commands to avoid running through the colorizer

- fix logout message - make instant action when event occurs

- add help command

  > help \[topic\]

- add ability to look at players

  > look at <player\>

- consider ways to improve true color -> 256 color degradation (avoiding grays)

- support UTF-8 clients

- add regions

- dedupe object set/clear flags

- add some pizzaz for players who are teleported on room deletion so they have some idea about what is happening

- fix race condition on shutdown preventing some players from receiving goodbye

- add immortal flag for players

- restrict immortal commands to players with the immortal flag

- remove unique object_id constraint on room_objects/player_objects - objects as prototypes

- add object quantity and associated manipulation commands

- add object init scripts

- allow state machines to be created and attached from an 'init' script
