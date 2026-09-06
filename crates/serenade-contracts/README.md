# serenade-contracts

Stable traits for generic persistence and cross-cutting ports: unit of work,
pagination, shared errors, and persist-param helpers.

Zero database crate dependencies. Applications define their own domain
repository traits and implement adapters with SQLx, SeaORM, or other stores.
