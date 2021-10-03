---
title: "Web Api"
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

# Authentication

Token-based authentication API for authorizing other API's use.

## POST /auth/login

Logs a player in and retrieves an access and refresh token.

```
in: {
  username: String,
  password: String
}

out: {
  access_token: String,
  refresh_token: String
}
```

## POST /auth/refresh

Requests a new token pair if the access token has expired.

```
in: {
  refresh_token: String
}

out: {
  access_token: String,
  refresh_token: String
}
```

## POST /auth/logout

Logs the player out, removing all stored tokens.

Uses bearer authentication.

```
headers:
Authorization: Bearer <access token>

in: {}
out: {}
```

# Scripting

## POST /scripts/create

Creates and compiles a new script, returning any compilation errors.

Uses bearer authentication.

```
headers:
Authorization: Bearer <access token>

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

Uses bearer authentication.

```
headers:
Authorization: Bearer <access token>

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

Uses bearer authentication.

```
headers:
Authorization: Bearer <access token>

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

Uses bearer authentication.

```
headers:
Authorization: Bearer <access token>

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

Uses bearer authentication.

```
headers:
Authorization: Bearer <access token>

in: {
  name: String
}

out: {}
```

# Websockets

Allows spinning up a websocket connection to ReMUD.

## /ws

The websocket endpoint. Supports upgrading properly formed client requests to websockets.

**From client to server:**

```
{
  "type": "game",
  "data": {
    "message": String
  }
}
```

**From server to client:**

```
{
  "type": "game",
  "data": {
    "is_prompt": true | false,
    "segments": [
        {
            t: "t",
            d: {
                text: "the message",
            }
        },
        {
            t: "cs",
            d: {
                color: "ffffff",
            }
        },
        {
            t: "ce",
            d: {},
        },
    ]
  }
}
```
Each message from the server is a line that is to be displayed. It's broken up into segments that represent how to color pieces of the line. 
`is_prompt` indicates whether the line is meant to be a player prompt - on the client, text is appended to the most recent prompt. 

- `t` segments contain the actual text, including whitespace.
- `cs` segments are 'color start' indicators and contain the color to apply to the subsequent segments
- `ce` is the 'color end' segment, and indicates to stop the previously applied color. 

