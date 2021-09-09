- add message command to send private messages to other players

  > message <player\> <contents\>

- add who command to list online players

  > who

- allow looking into adjacent rooms

  > look <direction\>

- add help command

  > help \[topic\]

- add emote command

  > me <emote\>

- add unlink command to remove room links

  > room unlink <direction\>

- add command to create objects

  > object new

- add item manipulation commands for object keywords, short desc, and long desc

  > object <id\> keywords <keyword(s)\>

  > object <id\> short <short description\>

  > object <id\> long <long description\>

- allow players to look at objects and see long desc via

  > look at <keyword(s)\>

- allow objects to be seen via short desc when looking in a room

- persist objects to database

- add object flags commands

  > object <id\> set <flag\>

  > object <id\> unset <flag\>

- add flag for fixed object that cannot be moved/picked up

- add flag for subtle object that should not be listed when the room is looked at

- give players an inventory, allow them to list its contents

  > inventory

- allow players to pick up object

  > get <keyword(s)\>

- allow players to drop object

  > drop <keyword(s)\>
