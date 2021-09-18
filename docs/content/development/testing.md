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
| script attach-pre object  |           |         |
| script attach-pre player  |           |         |
| script attach-pre room    |           |         |
| script attach object      |           |         |
| script attach player      |           |         |
| script attach room        |           |         |
| script detach-pre object  |           |         |
| script detach-pre player  |           |         |
| script detach-pre room    |           |         |
| script detach object      |           |         |
| script detach player      |           |         |
| script detach room        |           |         |
| player info               |           |         |
| object info               |           |         |
| room info                 |           |         |


## Script triggers
| Trigger                   | Entity    | Working ? | Comment |
| ------------------------- | --------- | --------- | ------- |
|   |           |         |
|   |           |         |
|   |           |         |
|   |           |         |
|   |           |         |
|   |           |         |
|   |           |         |
|   |           |         |
|   |           |         |
|   |           |         |
|   |           |         |
|   |           |         |
|   |           |         |
|   |           |         |
|   |           |         |


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
