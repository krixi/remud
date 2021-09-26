---
title: "Script Web Api"
date: 2021-09-17T12:54:12-07:00
weight: 999
summary: "The contract for the web API that manages scripts."
author: "Shaen"
tags: ["api", "web"]
---

Runs on port 2080.

Returns appropriate HTTP status codes on error:

- Bad trigger name: bad request (400)
- Bad script name: bad request (400)
- Duplicate name: conflict (409)
- Script not found: not found (404)

## POST /scripts/create

Creates and compiles a new script, returning any compilation errors.

```
in: {
  name: String,
  trigger: String,
  code: String
}

out: {
  error?: {
    line?: Number,
    position?: Number,
    message: String
  }
}
```

## POST /scripts/read

Retrieves a script and its compilation status.

```
in: {
  name: String
}

out: {
  name: String,
  trigger: String,
  code: String
  error?: {
    line?: Number,
    position?: Number,
    message: String
  }
}
```

## POST /scripts/read/all

Retrieves a list of all scripts including their length and compilation status.

```
in: {}
out: {
  scripts: [
    {
      name: String,
      trigger: String,
      lines: Number,
      error?: {
        line?: Number,
        position?: Number,
        message: String
      }
    }
  ]
}
```

## POST /scripts/update

Updates a script returning any compilation errors.

```
in: {
  name: String,
  trigger: String,
  code: String
}

out: {
  error?: {
    line?: Number,
    position?: Number,
    message: String
  }
}
```

## POST /scripts/delete

Deletes a script.

```
in: {
  name: String
}

out: {}
```
