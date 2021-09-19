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
Each script must have a unique name - they are associated with specific entities by name by invoking the
[immortals script commands](../immortals#scripts). 

## Triggers

Scripts are triggered to run when a specific kind of action occurs. Triggers are scoped to each room. 
Put another way, when an action occurs in a room, any script in the room that has a trigger for that action will execute.

For example, if an object has an attached script with a Drop trigger, that script will execute anytime anything drops something into the room.


### pre-attach vs attach

Scripts can be attached to run either _before_ or _after_ their triggering action. 

A nuance of this behavior is apparent with the `Move` trigger: You need to use pre-attach for triggering in the room being left, 
and attach to trigger in the room you arrive in.

Additionally, scripts that run in pre-attach can set `allow_action = false;` to prevent the action from continuing.

## Example

```
if EVENT.actor != SELF {
  let name = WORLD.get_name(EVENT.actor);
  WORLD.say(SELF, `Hello there, ${name}. I am a bear! Rawr`);
}
```

This example demonstrates some of the constants that are exposed to script, which you'll
use to implement behavior.

## `SELF`
The entity that this script is attached to.

---

## `EVENT`
This is the event that triggered the script.

**Fields:**

`actor` - The entity that sent the event.

---

## `WORLD`
The handle to the world and its APIs.

**Methods:**

`get_name(entity)` - Returns the name of the given entity, if it has one. 

`say(entity, text)` - Causes the entity to say the given text. 

---

