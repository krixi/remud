- allow script actors to queue events (say)

- add immortal commands to attach scripts to entities

  > script <name> attach-pre [object|player|room] <id/name>
  > script <name> attach [object|player|room] <id/name>

- make ScriptName require ascii names (or at least reject whitespace)

- add help command

  > help \[topic\]

- add ability to look at players

  > look at <player\>

- add colors

- add regions

- add agents

- allow agents to wander around regions

- refactor object.container to Location and Container components

- dedupe object set/clear flags

- add some pizzaz for players who are teleported on room deletion so they have some idea about what is happening
