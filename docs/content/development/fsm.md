---
title: "FSM"
date: 2021-09-21T17:38:12-07:00
weight: 4
summary: "Information about FSMs and scripting."
author: "Shane"
tags: ["scripting", "fsm"]
---

You can instantiate or remove an FSM in script using the following APIs. 

```
let builder = fsm_builder();

builder.add_state(StateId::WANDER, #{ region: "street"});

builder.add_state(StateId::CHASE, #{ timeout: 10000});

SELF.push_fsm(builder);

SELF.pop_fsm();
```

Parameters are state ID, and an [object map](https://rhai.rs/book/language/object-maps.html) to configure the behavior of the state. 
The first state added becomes the initial state in the FSM. 