- consider ways to improve true color -> 256 color degradation (avoiding grays). CIE-LAB space?

- support UTF-8 clients

- add some pizzaz for players who are teleported on room deletion so they have some idea about what is happening

- add object quantity and associated manipulation commands

- add combat

- add reputation

- deduplicate object/prototype systems

- add hp to prompt

- add customizable prompts

- add a resolve step for events to resolve targets so scripts have access to target information (get, drop, look at as examples)

- password verification is slow - do it in another task

- make objects.prototype_id a foreign key with on delete cascade

- figure out how to factor out Engine.process_input (needed for character creation)
