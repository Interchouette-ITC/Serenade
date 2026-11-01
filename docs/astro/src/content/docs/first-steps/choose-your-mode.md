---
title: Choose a web mode
description: Run the current HTML, SPA, SSG, or Universal example from the workspace.
---

Serenade has four web examples. They demonstrate different rendering and hosting arrangements;
none is a single required application stack.

Run commands from the repository root.

| Mode | Command | What it demonstrates |
| --- | --- | --- |
| HTML | `make html` | Server-rendered MyFeed example on `127.0.0.1:8080` |
| SPA | `make spa` | Client-rendered Leptos app built with Trunk |
| SSG | `make ssg` | Prerendered pages plus browser hydration |
| Universal | `make universal` | Leptos frontend served by a running Serenade backend |

HTML and Universal both default to port `8080`; stop one before running the other. SPA and SSG
produce static build output. Preview the generated files using the commands documented in
the [front modes guide](../../concepts/front-modes/).

For folder layout, output paths, and current limitations, see the full
[front modes guide](../../concepts/front-modes/).
