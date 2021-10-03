---
title: "Immortals"
date: 2021-09-16T13:12:14-07:00
weight: 1
summary: "Comprehensive list of abilities available to immortals."
author: "krixi"
tags: ["immortals", "help"]
---

Immortals are given special powers in CitySix.

# Server

### `shutdown`

This immediately shuts down the CitySix server. All connected citizens will be disconnected.

# Movement

### `teleport <room_id>`

Teleports you instantly to the room with the specified id.

# Prototypes

Prototypes are the blueprints from which objects are made. Objects by default inherit
all of their properties from a prototype, though this can be overridden on a per-object basis.

### `prototype new`

Creates a new prototype.

### `prototype list`

Lists all existing prototypes by ID and name.

### `prototype <id> info`

Displays all available information about the prototype.

### `prototype <id> name <text>`

Changes the name of the prototype. Names are how objects are commonly displayed in
short form to players. The name given should start with a lowercase letter (unless
it's a proper noun), and should not end with punctuation.

### `prototype <id> description <text>`

Changes the description of the prototype. Descriptions are shown when a player looks
at an object. This field is treated as prose, so it should consist of complete sentences
and may contain paragraphs.

### `prototype <id> keywords (set|add|remove) <text>`

Changes the keywords of the prototype. Keywords are how players interact with objects.
They should be similar to the name and possibly include some useful words from the
description for disambiguation.

### `prototype <id> set <flags>` / `prototype <id> unset <flags>`

Sets or clears flags on the prototype. Flags set boolean properties. See below for a
description of object flags.

# Objects

Objects are the things in the world (npcs, items, etc.). They inherit all properties
from their prototype by default. Many objects can reference the same prototype. Object
properties can be overridden from the prototypes on a case by case basis.

### `object new <prototype id>`

Creates a new object from the given prototype. By default this object will inherit all
properties from the prototype.

### `object <id> set <flags>` / `object <id> unset <flags>`

Sets or unsets flags on the object. `<flags>` is a space separated list of strings:

- `fixed` - cannot be picked up
- `subtle` - does not show up in `look` command

### `object <id> info`

Shows info about the object.

### `object <id> name <text>`

Sets the object's name to the specified text.

### `object <id> desc <text>`

Sets the object's description to the specified text.

### `object <id> keywords <space separated list>`

Sets the object's list of keywords. These keywords should be referenced in the object's
name and description.

### `object <id> inherit [name] [desc] [flags] [keywords] [scripts]`

Sets the object to inherit the specified field from its prototype. This is useful when
the field was overridden on the object but needs to be changed back to inherit.

### `object <id> init`

Re-initializes the object. This removes all script data, FSMs, and timers from the object
and re-runs any attached init scripts.

### `object <id> errors <script name>`

Checks if the named script has had any execution errors on the object. If so, displays the
script's code with the error location highlighted as well as the details on the error that
occurred.

### `object <id> remove`

Removes the specified object.

# Players

### `player <name> info`

Shows info about the player with the given name

### `player <name> set <flags>` / `player <name> unset <flags>`

Sets or clears flags on the player. Flags are:

- `immortal` - grants the player access to immortal commands

### `player <name> init`

Clears all script data, FSMs, and timers from the player before re-running any attached init scripts.

### `player <id> errors <script name>`

Checks if the named script has had any execution errors on the player. If so, displays the
script's code with the error location highlighted as well as the details on the error that
occurred.

# Rooms

These commands implicitly assume the current room as the ID of the room you wish to act upon.

### `room info`

Shows info about current room.

### `room new [dir]`

Creates a new room, optionally in the specified direction. If you specify a direction, links
from the current room to the new room (and back) will automatically be created.

### `room desc <text>`

Sets the description of the room. This field is treated as prose,
so should consist of complete sentences and may contain paragraphs.

### `room link <dir> <id to link to>`

Links the current room to the given room via the given direction.

### `room regions (set|add|remove) <space separated list>`

Adds or removes the list of regions to or from the current room, respectively.

### `room unlink <dir>`

Unlinks the current room from the specified direction.

### `room init`

Removes all script data, FSMs, and timers from the current room before re-running any attached init scripts.

### `room errors <script name>`

Checks if the named script has had any execution errors on the room. If so, displays the
script's code with the error location highlighted as well as the details on the error that
occurred.

### `room remove`

Removes the current room. You'll be teleported to the void room.

# Scripts

Scripts are created through the web-client. Once created, they can be attached and detached from entities with the following commands
([learn more]({{< relref "./scripting" >}})).

### `script <name> attach-init [prototype|object|player|room] <id/name>`

Attaches an init script to the entity. Init scripts are run when the entity is loaded or when the type-specific init command is executed on the entity. Object init scripts are also run when new objects are created, if assigned via prototype.

### `script <name> attach-pre [prototype|object|player|room] <id/name>`

Attaches a pre-event script to an entity. These are triggered by events before the action is processed and can be used to deny the action.

### `script <name> attach-post [prototype|object|player|room] <id/name>`

Attaches a post-event script to an entity. These are triggered by events after the action is processed and are used to respond to actions.

### `script <name> attach-timer <timer name> [prototype|object|player|room] <id/name>`

Attaches a timer script to an entity. These execute when the named timer finishes. Without the timer, these scripts never execute.

### `script <name> detach [prototype|object|player|room] <id/name>`

Detaches a script by name from the entity.
