- add ability to look at players

  > look at <player\>

- add player descriptions

- consider ways to improve true color -> 256 color degradation (avoiding grays). CIE-LAB space?

- support UTF-8 clients

- dedupe set name/description systems (common types)

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

- allow state machines to be created and attached from an 'init' script

- try to re-use swapped vecs to avoid allocating more

- add combat

- add reputation

- colorize room look

- add ability to queue actions from scripts to occur after a certain amount of time has passed

- make object/room new persist objects take all parameters from caller

let fsm = build_fsm();

let wander = new_wander_state(#{ region: "street", min_wait: 10000, max_wait: 60000});

fsm.add_state();

SELF.attach(fsm.build());
