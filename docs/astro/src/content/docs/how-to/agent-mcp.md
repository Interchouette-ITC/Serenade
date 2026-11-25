---
title: "Coding agents and MCP"
description: "Use source-backed Serenade and Rangular documentation with coding agents."
---

The standalone `serenade-rangular-mcp` serves embedded Serenade and Rangular
documentation to MCP-capable coding agents. The documentation and examples
identify the product, version, source revision, and original path. Each binary
contains one manually selected source snapshot per product; it does not fetch
documentation from this website.

## Local stdio

Install the binary, then configure the agent to start it:

```sh
cargo install --locked serenade-rangular-mcp
```

```json
{
  "servers": {
    "serenade-rangular": {
      "type": "stdio",
      "command": "serenade-rangular-mcp",
      "args": ["--profile", "all"]
    }
  }
}
```

Profiles are `serenade`, `rangular`, and `all`. The MCP supports listing,
searching and reading documents and exposes them as resources. Rangular
template validation uses the released parser, which can differ from the
development documentation; it is not an AOT compilation or an application
build. Local stdio also exposes tools to scaffold a Serenade application and
run Cargo check, build or test. These can write files and execute build scripts
or tests; use a client that requests approval. The agent can also run
`serenade`, `serenade-trunk`, Cargo, and other commands locally.

## Streamable HTTP

For local HTTP:

```sh
serenade-rangular-mcp --transport http --profile all
```

Connect to `http://127.0.0.1:8765/mcp`. Public hosting is not deployed by
this project. A future service at `mcp.serenade.rs` may take roughly 15 seconds
to wake up; configure client timeouts and retry/reconnect where supported.
Public HTTP is read-only and does not access the caller's project.

## Markdown without MCP

The static documentation build publishes original Markdown separately from
the MCP. Use the explicit `.md` URLs; requesting the HTML page with
`Accept: text/markdown` does not select Markdown. For example:

- [Serenade recipes](/markdown/serenade/docs-dev/RECIPES.md)
- [Serenade front modes](/markdown/serenade/docs-dev/FRONT-MODES.md)
- [Rangular guide](https://raw.githubusercontent.com/Interchouette-ITC/rangular/dev/docs/README.md)
- [Rangular specification](https://raw.githubusercontent.com/Interchouette-ITC/rangular/dev/docs/SPEC.md)
- [Markdown index](/markdown/index.json)

The website's documentation can differ from the snapshot embedded in an
installed MCP binary. Check the MCP result's version and revision before
applying its examples. The Markdown index records a SHA-256 hash of each
published file.
