---
title: Architecture
description: How Serenade adapts Symfony’s component architecture to Rust without porting its APIs.
---

# What if Symfony’s architectural ideas were built for Rust?

Symfony demonstrated how an application framework can be composed from independent components,
coordinated by a kernel, and extended with bundles. Serenade takes inspiration from those ideas
and implements its own Rust-native architecture.

Serenade is an independent Rust project—not a PHP port, a Symfony compatibility layer, or an
affiliated Symfony product. Cargo owns dependency management. Rust traits and types define the
contracts. The application chooses its server and persistence strategy.

## Four parts. One application.

| Part | Responsibility |
| --- | --- |
| **Kernel** | Owns application lifecycle: bundle registration, compilation, boot, and shutdown. |
| **Bundles** | Compose related services, configuration, routes, subscribers, and commands. |
| **Components** | Provide focused capabilities such as DI, events, HTTP, console, security, and forms. |
| **Adapters** | Connect framework contracts to infrastructure choices such as Actix Web and Axum. |

The application assembles the pieces it needs. Bundles are the composition mechanism; components
can also be used independently when an application does not need the full kernel.

## Rust-native choices

- **Cargo, not a second package manager.** Recipes and the CLI assist with composition; Cargo
  remains responsible for dependencies.
- **Contracts, not a mandated database.** Applications choose SQLx, SeaORM, Diesel, or another
  persistence strategy. Serenade does not ship an ORM.
- **Adapters at the boundary.** The HTTP foundation is distinct from Actix and Axum integration.
- **Product domain stays with the product.** Commerce, CMS, and other domain features are not
  part of the framework kernel.

## Maturity

Serenade is pre-1.0 and evolving. The architecture describes the framework’s direction; it does
not mean every component is equally complete or production-ready. Review the
[kernel documentation](../kernel/) and examples before adopting a crate.

For the detailed design, see the source-backed [framework vision](../vision/)
and [kernel guide](../kernel/).
