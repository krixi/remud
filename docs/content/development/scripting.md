---
title: "Scripting"
date: 2021-09-16T14:17:25-07:00
weight: 2
summary: "Scripting language integration."
author: "krixi"
tags: ["api", "scripts"]
---

ReMUD uses [Rhai](https://rhai.rs/) as the scripting engine, 
providing a dynamic control layer over the rust-backend primitives. 

Scripts in ReMUD are invoked when they are triggered. They are managed by the web-console. 
Each script must have a unique name - they are associated with specific entities by name. 

# Example

```
if EVENT.actor != SELF {
  let name = WORLD.get_name(EVENT.actor);
  WORLD.say(SELF, `Hello there, ${name}. I am a bear! Rawr`);
}
```

This example demonstrates some of the constants that are exposed to script, which you'll
use to implement behavior.


## `EVENT`
This is the event that triggered the script.


## `WORLD`
The handle to the world and its APIs.

## `SELF`
The entity that this script is attached to. 
