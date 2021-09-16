---
title: "Commands"
date: 2021-09-16T12:59:40-07:00
weight: 1
summary: "Comprehensive list of commands available to citizens."
author: "krixi"
tags: ["help"]
---

As a citizen of CitySix, you interact with the world by issuing commands at your terminal. These commands are described 
below, organized by function. 

# Communication

## `emote <text>` / `; <text>`
Express an emotion. Only others at your current location will see your emote.

## `say <text>`  / `' <text>`
Say something aloud. Only others at your current location will hear you.

## `send <name> <text>`
Send a message directly to another citizen. Capitalization is important when specifying the name to send to.


# Inventory
## `drop <keywords>`
Drops the first item matched by the specified keywords. The item is moved from your inventory
into the room in which you drop it. 

## `get <keywords>`
Picks up the first item matched by the specified keywords. The item is placed into your inventory.

## `inventory`
Lists the items you are currently carrying.

# Movement
## `north` / `south` / `east` / `west` / `up` / `down`
These will cause you to move to the location in the specified direction. 
They will only work if there is an exit from your current location in that direction.


# Observation
## `exits`
Shows you the ways you can leave your current location.

## `look`
Causes you to examine the current location.

## `look at <keywords>`
Causes you to closely examine the first object that matches the specified keywords.
This object can be in your inventory, or somewhere in the location you are currently in.

## `who`
Displays a list of other citizens who are currently connected.


