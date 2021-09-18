---
title: "Test plan"
date: 2021-09-16T15:21:38-07:00
weight: 3
summary: "For when you need to manually verify things work"
author: "krixi"
tags: ["testing"]
---

## Script attach & persist

| Command                   | Working ? | Comment |
| ------------------------- | --------- | ------- |
| script attach-pre object  |   ✅      |         |
| script attach-pre player  |   ✅      |         |
| script attach-pre room    |   ✅      |         |
| script attach object      |   ✅      |         |
| script attach player      |   ✅      |         |
| script attach room        |   ✅      |         |
| script detach object      |   ✅      |         |
| script detach player      |   ✅      |         |
| script detach room        |   ✅      |         |
| player info               |   ✅      |         |
| object info               |   ✅      |         |
| room info                 |   ✅      |         |
| `allow_action`            |   ✅      |         |


## Script triggers

Bug: Same script shouldn't be able to be registered twice for the same trigger. 

| Trigger       | Entity    |  ?        | Comment |
| ------------- | --------- | --------- | ------- |
|  Drop         | object    |   ✅      |         |
|  Emote        | object    |   ✅      |         |
|  Exits        | object    |   ✅      |         |
|  Get          | object    |   ✅      |         |
|  Inventory    | object    |   ✅      |         |
|  Look         | object    |   ✅      |         |
|  LookAt       | object    |   ✅      |         |
|  Move         | object    |   ✅      |         |
|  Say          | object    |   ✅      |         |
|  Send         | object    |   ✅      |         |
| ---           | ---       | ---       | ---     |
|  Drop         | player    |   ✅      |         |
|  Emote        | player    |   ✅      |         |
|  Exits        | player    |   ✅      |         |
|  Get          | player    |   ✅      |         |
|  Inventory    | player    |   ✅      |         |
|  Look         | player    |   ✅      | Automatic look when entering a room does not trigger this. |
|  LookAt       | player    |   ✅      |         |
|  Move         | player    |   ✅      |         |
|  Say          | player    |   ✅      |         |
|  Send         | player    |   ✅      |         |
| ---           | ---       | ---       | ---     |
|  Drop         | room      |   ✅      |         |
|  Emote        | room      |   ✅      |         |
|  Exits        | room      |   ✅      |         |
|  Get          | room      |   ✅      |         |
|  Inventory    | room      |   ✅      |         |
|  Look         | room      |   ✅      | Automatic look when entering a room does not trigger this. |
|  LookAt       | room      |   ✅      |         |
|  Move         | room      |   ✅      |         |
|  Say          | room      |   ✅      |         |
|  Send         | room      |   ✅      |         |

## EventAction refactor 

| Command           | Working? | Comment |
| ------------------| ------- | ------- |
| emote             |  ✅     |  |
| say               |  ✅     |  |
| send              |  ✅     |  |
| inventory         |  ✅     |  |
| drop              |  ✅     |  |
| get               |  ✅     |  |
| north             |  ✅     |  |
| south             |  ✅     |  |
| east              |  ✅     |  |
| west              |  ✅     |  |
| up                |  ✅     |  |
| down              |  ✅     |  |
| exits             |  ✅     |  |
| look              |  ✅     |  |
| look at           |  ✅     |  |
| who               |  ✅     |  |
| shutdown          |  ✅     | race condition around sending happy shutdown message to players |
| teleport          |  ✅     |  |
| object new        |  ✅     |  |
| object info       |  ✅     |  |
| object name       |  ✅     | still is "short" in parser |
| object desc       |  ✅     | still is "long" in parser |
| object keywords   |  ✅     |  |
| object set        |  ✅     |  |
| object unset      |  ✅     |  |
| object remove     |  ✅     |  |
| player info       |  ✅     |  |
| room info         |  ✅     |  |
| room new          |  ✅     |  |
| room new dir      |  ✅     |  |
| room link         |  ✅     |  |
| room unlink       |  ✅     |  |
| room remove       |  ✅     |  |
