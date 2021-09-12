- change persist objects to take exclusively IDs and remove the World parameter

- execute persists in their own async task

- add help command

  > help \[topic\]

- add unlink command to remove room links

  > room unlink <direction\>

- add ability to look at players

  > look at <player\>

- add object flags commands

  > object <id\> set <flag\>

  > object <id\> unset <flag\>

- add flag for fixed object that cannot be moved/picked up

- add flag for subtle object that should not be listed when the room is looked at
