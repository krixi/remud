---
title: "Script Web Api"
date: 2021-09-17T12:54:12-07:00
draft: true
weight: 999
summary: ""
author: ""
tags: [""]
---

Runs on port 2080.

## POST /scripts/create

```
in: {
  name: String,
  trigger: String,
  code: String
}

out: {}
```

## POST /scripts/read

```
in: {
  name: String
}

out: {
  name: String,
  trigger: String,
  code: String
}
```

## POST /scripts/read/all

```
in: {}
out: {
  scripts: [
    {
      name: String,
      trigger: String,
      code: String
    }
  ]
}
```

## POST /scripts/update

```
in: {
  name: String,
  trigger: String,
  code: String
}

out: {}
```

## POST /scripts/delete

```
in: {
  name: String
}

out: {}
```
