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
[immortal scripts commands]({{< relref "./immortals#scripts" >}}).

## Triggers

Scripts can be triggered in various ways: by player action, entity initialization, or timers.

### Action Event Triggers

Scripts can be triggered to run when a specific kind of action occurs. Action event triggers
are scoped to each room. Put another way, when an action occurs in a room, any script in
the room that has a trigger for that action will execute.

For example, if an object has an attached script with a Drop trigger, that script will
execute anytime anything drops something into the room.

Scripts can be attached to run either _before_ or _after_ their triggering action.

A nuance of this behavior is apparent with the `Move` trigger: You need to use `attach-pre` for triggering in the room being left,
and `attach-post` to trigger in the room you arrive in.

Additionally, scripts that run in `attach-pre` can set `allow_action = false;` to prevent the action from continuing.

Scripts executed via action triggers will have the event object available for inspection as
the `EVENT` constant.

### Init Triggers

Scripts can be attached as initialization scripts with `attach-init`. Scripts attached this
way will be run when the entity is loaded or when its `init` command is run. Additionally,
new objects created from prototypes will run their init scripts immediately.

### Timer Triggers

Scripts can also be attached to execute when a specific timer elapses with `attach-timer`. The
timer must be named when the script is attached, and only that timer will trigger the script.

## Example

```
if WORLD.is_player(EVENT.actor) {
  let name = WORLD.name(EVENT.actor);
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

`emote_after(duration, text)` - Causes self to perform the given emote after the duration has elapsed.

`message(text)` - Sends the message to the current room. No additional text is added to the message.

`message_after(duration, text)` - Sends the message to the current room afer the duration has elapsed.

`say(message)` - Causes self to say the given message.

`say_after(duration, message)` - Causes self to say the given message after the duration has elapsed

`send(recipient, message)` - Causes self to send the message to the provided recipient. Recipient must be a player's name as a string.

`send(duration, recipient, message)` - Causes self to send the message to the provided recipient after the duration has elapsed.

`whisper(entity, text)` - Sends the message to target player. No additional text is added to the message.

`whisper_after(duration, entity, text)` - Sends the message to target player afer the duration has elapsed.

`timer(name, duration)` - Creates a new timer for this entity with the specified duration.
When the timer duration elapses, any timer scripts attached to this timer name will be
executed, then the timer will be removed.

`timer_repeating(name, duration)` - Creates a new timer for this entity with the specified
duration. This timer will not be removed when its duration elapses and will trigger any
attached and matching timer scripts whenever it elapses.

`get(key)` - Retrieves a value from the entity's shared script data.

`set(key, value)` - Sets a value into the entity's shared script data. This data is not persisted.

`remove(key)` - Removes a value from the entity's shared script data.

`push_fsm(state_machine)` - Pushes a new state machine onto the entity's FSM stack. The
top machine is executed.

`pop_fsm()` - Pops the top state machine from the entity's FSM stack.

---

## `EVENT`

This is the event that triggered the script.

**Fields:**

`actor` - The entity that sent the event.

`is_emote` - True if the event is an Emote event, false otherwise.

`is_move` - True if the event is a Move event, false otherwise.

`emote` - Retrieves the emote of an Emote event, or unit if not.

`direction` - Retrieves the movement direction of a Move event, or unit if not.

---

## `WORLD`

The handle to the world and its APIs.

**Methods:**

`is_player(entity)` - Returns true if the provided entity is a player, false otherwise.

`is_room(entity)` - Returns true if the provided entity is a room, false otherwise.

`is_object(entity)` - Returns true if the provided entity is a object, false otherwise.

`name(entity)` - Returns the name of the given entity, or unit if it doesn't have one. Players and objects have names.

`description(entity)` - Returns the description of the given entity, or unit if it doesn't have one. Rooms and objects have descriptions.

`keywords(entity)` - Returns the list of keywords for the given entity, or unit if it doesn't have keywords. Objects have keywords.

`location(entity)` - Returns the location of the given entity, or unit if it isn't in a room. Players always have locations, and objects have locations when they are in a room.

`container(entity)` - Returns the container of the given entity, or unit if it is not in a container. Objects have containers when they are being carried by a player.

`contents(entity)` - Returns the contents of the given container, or unit if it is not a container. Players and rooms are containers and can hold objects.

`players(entity)` - Returns the players in the given room, or unit if it isn't a room.

`object_new(prototype_id)` - Creates a new instance of the given prototype, and drops it on the floor of the current room.   

---

## `Library`

Functions included in addition to the Rhai standard libraries.

### `time`

`ms(i64)` - Returns a `Duration` with length as specified in milliseconds

`secs(i64)` - Returns a `Duration` with length as specified in seconds

### `random`

`chance(probability)` - Randomly determines if probability occurred and returns a boolean.
For example, a probability of `.2` indicates a 20% chance and will return true one fifth
of the time.

`choose(array)` - Randomly selects and returns an item from the specified array.

`range(start, end)` - Randomly returns a value between start and end, inclusive.


### state machines

`let builder = fsm_builder()` - creates a new FSM builder. See [the page on FSM's]({{< relref "./fsm" >}}) 
for details on how to configure these objects.