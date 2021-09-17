---
title: "Script Web Api"
date: 2021-09-17T12:54:12-07:00
draft: true
weight: 999
summary: ""
author: ""
tags: [""]
---

## POST /scripts/create

```
in: {
  name: String,
  trigger: String,
  code: String
}

out: {
  id: String
}
```

## POST /scripts/read

```
in: {
  id: String
}

out: {
  id: String
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
      id: String
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
  id: String,
  name: String,
  trigger: String,
  code: String
}

out: {}
```

## POST /scripts/delete

```
in: {
  id: String
}

out: {}
```
