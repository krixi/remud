- consider ways to improve true color -> 256 color degradation (avoiding grays). CIE-LAB space?

- support UTF-8 clients

- add some pizzaz for players who are teleported on room deletion so they have some idea about what is happening

- fix race condition on shutdown preventing some players from receiving goodbye

- add immortal flag for players

- restrict immortal commands to players with the immortal flag

- implement object prototypes

  - add prototypes table (convert all current objects to prototypes)
  - make object table fields nullable
  - alter systems to support checking object then prototype
  - add prototype commands

- add object quantity and associated manipulation commands

- add object init scripts

- try to re-use swapped vecs to avoid allocating more

- add combat

- add reputation

- add ability to queue actions from scripts to occur after a certain amount of time has passed

- allow state machines to be created and attached from an script
