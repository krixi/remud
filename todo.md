- allow script actors to queue events (say)

- add http script management endpoint

  - list script w/status (compiled, error)

  - script CRUD

- add immortal commands to attach scripts to entities

  > script <name> attach-pre [object|player|room] <id/name>
  > script <name> attach [object|player|room] <id/name>

- execute persists in their own async tasks

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
