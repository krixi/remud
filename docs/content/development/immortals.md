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

# Objects

### `object new`

### `object <id> set <flags>` / `object <id> unset <flags>`

Sets or unsets flags on the object. `<flags>` is a space separated list of strings:

- `fixed` - cannot be picked up
- `subtle` - does not show up in `look` command

### `object <id> info`

Shows info about the object.

### `object <id> name <text>`

Sets the object's name to the specified text.
The name given should start with a lowercase letter (unless it's a proper noun),
and should not end with punctuation.

### `object <id> desc <text>`

Sets the object's description to the specified text. This field is treated as prose,
so should consist of complete sentences and may contain paragraphs.

### `object <id> keywords <space separated list>`

Sets the object's list of keywords. These keywords should be referenced in the object's
name and description.

### `object <id> remove`

Removes the specified object.

# Players

### `player <name> info`

Shows info about the player with the given name

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

### `room region <add/remove> <space separated list>`

Adds or removes the list of regions to or from the current room, respectively.

### `room unlink <dir>`

Unlinks the current room from the specified direction.

### `room remove`

Removes the current room. You'll be teleported to the void room.

# Scripts

Scripts are created through the web-client. Once created, they can be attached and detached from entities with the following commands
([learn more]({{< relref "./scripting" >}})).

### `script <name> attach-pre [object|player|room] <id/name>`

### `script <name> attach [object|player|room] <id/name>`

### `script <name> detach [object|player|room] <id/name>`
