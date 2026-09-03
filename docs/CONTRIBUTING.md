# Contributing to Serenade

Thank you for improving the Serenade framework. This repo ships **kernel contracts and components** consumed by applications such as commerce products. Keep framework code free of product domain types and ORM dependencies.

## Before you open a PR

1. Read `docs-dev/` for architecture intent.
2. Run the full local gate:

```bash
make ci
```

3. One concern per PR. Draft until the slice is complete.
4. Every public item needs rustdoc. Add or extend unit tests for behavior you introduce.

## Toolchain

- Rust stable (see `rust-version` in the workspace `Cargo.toml`).
- Integration branch: `dev` on the org repo.
- Feature branches land via PR from the worker fork.

## Quality bar

| Gate | Command |
| --- | --- |
| Format + Clippy | `make lint` |
| Tests | `make test` |
| Rustdoc | `make doc` |
| Full CI slice | `make ci` |

Clippy uses `-D warnings` with the pedantic and nursery groups. Do not add `#[allow(clippy::too_many_arguments)]`, `too_many_lines`, or `dead_code`.

## Make layout

The root `Makefile` includes fragments under `make/`:

| File | Role |
| --- | --- |
| `make/common.mk` | Shared variables (`ROOT`, `CARGO`, `ARGS`) |
| `make/ci.mk` | Quality gates (`lint`, `test`, `doc`, `ci`) |
| `make/cli.mk` | Day-to-day aliases (`serenade`, `tui`, `console`, `demo`) |
| `make/docker.mk` | Container targets (placeholder until compose recipes land) |

Examples:

```bash
make serenade ARGS='recipe list'
make tui ARGS='--no-cargo'
make console ARGS='serenade:about'
```

Run `make help` for the full list.

## Documentation

- **Rust:** module docs (`//!`) on every crate root; `///` on every `pub` item.
- **Framework docs:** English in `docs-dev/`. No plan jargon or host-absolute paths in shipped text.
- **OpenAPI:** lives in product HTTP crates when routes exist; Serenade stays adapter-agnostic.

## Persistence boundary

Serenade defines traits in `serenade-contracts`. SQLx, SeaORM, and Diesel adapters belong in **application** repos, not here.

## Commits and PRs

Use conventional commits (`feat:`, `fix:`, `docs:`, `ci:`, …). PR body: **Summary** + **Test plan** only.

## Questions

Open a GitHub issue on `Interchouette-ITC/Serenade` for design questions that affect multiple crates.
