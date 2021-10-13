- consider ways to improve true color -> 256 color degradation (avoiding grays). CIE-LAB space?

- support UTF-8-capable telnet clients

- add object quantity and associated manipulation commands

- add ability to differentiate between objects with similar keywords (?)

- add object stats (by script? store in db?)

- add combat

- add reputation

- add hp to prompt

- add customizable prompts

- add script state to state machine

- think about how to make parameterizable scripts for common use cases - or do it in rust

- add a resolve step for events to resolve targets so scripts have access to target information (get, drop, look at as examples)

- password verification is slow - do it in another task

- consider cleaning up timers at the end of a tick

- finish building the dog

- add player kick, ban, unban

- support ssh (thrussh)

- support graceful shutdown handling via signals

- add persist feedback for immortal commands (?)

- add support for metrics (statsd -> telegraf -> influxdb -> grafana)

- add currency

Things that would benefit from a state machine:

- add change password command

- Any other place non-linear stateful input sequence is required (combat, minigames, hacking, etc...)

- check places with player names, make sure they allow spaces in their parsing (unlike player <name> info)
