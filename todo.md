- fix logout message - make instant action when event occurs

- add ability to look at players

  > look at <player\>

- add player descriptions

- consider ways to improve true color -> 256 color degradation (avoiding grays). CIE-LAB space?

- support UTF-8 clients

- dedupe object set/clear flags

- dedupe set name/description systems (common types)

- add some pizzaz for players who are teleported on room deletion so they have some idea about what is happening

- fix race condition on shutdown preventing some players from receiving goodbye

- add immortal flag for players

- restrict immortal commands to players with the immortal flag

- remove unique object_id constraint on room_objects/player_objects - objects as prototypes

- add object quantity and associated manipulation commands

- add object init scripts

- allow state machines to be created and attached from an 'init' script

- try to re-use swapped vecs to avoid allocating more

- add combat

- add reputation

- add world time/timers

- colorize room look
