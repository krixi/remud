---
title: "FSM"
date: 2021-09-21T17:38:12-07:00
draft: true
weight: 999
summary: "Information about FSMs and scripting."
author: "Shane"
tags: ["scripting", "fsm"]
---

Preliminary sketch of how binding an FSM to an object will work.

https://rhai.rs/book/language/object-maps.html

```
let fsm = build_fsm();

let wander = new_wander_state(#{ region: "street", min_wait: 10000, max_wait: 60000 });

fsm.add_state(wander);

SELF.push_fsm(fsm.build());

SELF.pop_fsm();
```
