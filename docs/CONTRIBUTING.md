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
| Coverage (lcov) | `make coverage` (needs `cargo llvm-cov`) |
| Rustdoc | `make doc` |
| Full CI slice | `make ci` |

Clippy uses `-D warnings` with the pedantic and nursery groups. Do not add `#[allow(clippy::too_many_arguments)]`, `too_many_lines`, or `dead_code`.

## Rust test DX

Runner stays **`cargo test`** (via `make test` / `make ci`). Prefer these workspace `dev-dependencies` when they fit:

| Crate | Use for |
| --- | --- |
| **rstest** | Parametrized cases and fixtures |
| **mockall** | Sync trait doubles when a real collaborator is heavy |
| **insta** | Stable JSON / YAML / text snapshots (`*.snap` committed; `*.snap.new` gitignored) |

Kernel boot / HTTP helpers live in **`serenade-testing`** (`SerenadeTestKernel`, `HttpTestClient`, re-exports of event harness helpers).

## Make layout

The root `Makefile` includes fragments under `make/`:

| File | Role |
| --- | --- |
| `make/common.mk` | Shared variables (`ROOT`, `CARGO`, `ARGS`) |
| `make/ci.mk` | Quality gates (`lint`, `test`, `doc`, `ci`) |
| `make/cli.mk` | Day-to-day aliases (`serenade`, `tui`, `console`, `demo`) |
| `make/docker.mk` | Container help stub (`docker-help`); compose targets land here when added |

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
- **Code of Conduct:** [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md)
- **Security policy:** [`SECURITY.md`](SECURITY.md)

## Persistence boundary

Serenade defines traits in `serenade-contracts`. SQLx, SeaORM, and Diesel adapters belong in **application** repos, not here.

## Commits and PRs

Use conventional commits (`feat:`, `fix:`, `docs:`, `ci:`, …). PR body follows
[`pull_request_template.md`](pull_request_template.md) (**Summary** + **Test plan** only).

## Questions

Use GitHub issue forms (Bug report / Feature request) when opening issues on
`Interchouette-ITC/Serenade` for design questions that affect multiple crates.
