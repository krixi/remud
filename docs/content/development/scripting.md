---
title: "Scripting"
date: 2021-09-16T14:17:25-07:00
weight: 2
summary: "Scripting language integration."
author: "krixi"
tags: ["api", "scripting"]
---

ReMUD uses [Rhai](https://rhai.rs/) as the scripting engine,
providing a dynamic control layer over the rust-backend primitives.

Scripts in ReMUD are invoked when they are triggered. They are managed by the web-console.
Each script must have a unique name - they are associated with specific entities by name by invoking the
[immortals script commands]({{< relref "./immortals#scripts" >}}).

## Triggers

Scripts are triggered to run when a specific kind of action occurs. Triggers are scoped to each room.
Put another way, when an action occurs in a room, any script in the room that has a trigger for that action will execute.

For example, if an object has an attached script with a Drop trigger, that script will execute anytime anything drops something into the room.

### pre-attach vs attach

Scripts can be attached to run either _before_ or _after_ their triggering action.

A nuance of this behavior is apparent with the `Move` trigger: You need to use `attach-pre` for triggering in the room being left,
and `attach` to trigger in the room you arrive in.

Additionally, scripts that run in `attach-pre` can set `allow_action = false;` to prevent the action from continuing.

## Example

```
if WORLD.is_player(EVENT.actor) {
  let name = WORLD.get_name(EVENT.actor);
  SELF.say(`Hello there, ${name}. I am a bear! Rawr`);
}
```

This example demonstrates some of the constants that are exposed to script, which you'll
use to implement behavior.

## `SELF`

The entity that this script is attached to.

**Fields:**

`entity` - Returns the entity of the thing the script is attached to.

**Methods:**

`emote(text)` - Causes self to perform the given emote.

`message(text)` - Sends the message to the room self is in as is. No additional text is added to the message.

`say(message)` - Causes self to say the given message.

`send(recipient, message)` - Causes self to send the message to the provided recipient. Recipient must be a player's name as a string.

---

## `EVENT`

This is the event that triggered the script.

**Fields:**

`actor` - The entity that sent the event.

---

## `WORLD`

The handle to the world and its APIs.

**Methods:**

`is_player(entity)` - Returns true if the provided entity is a player, false otherwise.

`is_room(entity)` - Returns true if the provided entity is a room, false otherwise.

`is_object(entity)` - Returns true if the provided entity is a object, false otherwise.

`get_name(entity)` - Returns the name of the given entity, or unit if it doesn't have one. Players and objects have names.

`get_description(entity)` - Returns the description of the given entity, or unit if it doesn't have one. Rooms and objects have descriptions.

`get_keywords(entity)` - Returns the list of keywords for the given entity, or unit if it doesn't have keywords. Objects have keywords.

`get_location(entity)` - Returns the location of the given entity, or unit if it isn't in a room. Players always have locations, and objects have locations when they are in a room.

`get_container(entity)` - Returns the container of the given entity, or unit if it is not in a container. Objects have containers when they are being carried by a player.

`get_contents(entity)` - Returns the contents of the given container, or unit if it is not a container. Players and rooms are containers and can hold objects.

`get_players(entity)` - Returns the players in the given room, or unit if it isn't a room.
