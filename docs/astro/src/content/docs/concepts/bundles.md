---
title: Bundles
description: Compose application services and framework integrations with Serenade bundles.
---

A bundle is Serenade’s unit of reusable, in-process application composition. It can declare
dependencies on other bundles and participate in the kernel lifecycle.

Bundles can contribute services, package configuration, routes, event subscribers, and console
commands through the framework’s extension points. The kernel orders bundles by their declared
dependencies before build and boot; shutdown happens in reverse dependency order.

The canonical [`demo-app`](../../first-steps/first-application/) pairs `DemoBundle` with
`FrameworkBundle`. It is a runnable reference, not a required application template.

For implementation details, dependency rules, extensions, and the public API boundary, read the
[bundle extensions guide](../bundle-extensions/).
