# Performance tooling

Two different tools for two jobs:

| Tool | Job |
| --- | --- |
| [`serenade-profiler`](PROFILER.md) (Web Debug Toolbar) | Per-request **browser** inspect in `dev`: routing, timing, logs, SQL, debug notes, views |
| [hotpath](https://hotpath.rs/) (optional app dependency) | Function / async / I/O **performance** reports (timing, allocations). Not a request toolbar |

Serenade does **not** hard-depend on hotpath in kernel or HTTP crates. Apps opt in.

## When to use which

- Debugging “why is this request wrong?” → enable the profiler toolbar.
- Finding “why is this function slow?” → enable hotpath features on the app binary.

## App recipe (optional features)

In an application `Cargo.toml` (example: `examples/MyFeed`):

```toml
[features]
default = []
hotpath = ["dep:hotpath"]
hotpath-alloc = ["hotpath", "hotpath/hotpath-alloc"]

[dependencies]
hotpath = { version = "0.28", optional = true, default-features = false }
```

Instrument with `#[cfg_attr(feature = "hotpath", hotpath::measure)]` on hot functions, and wrap the binary entry with hotpath’s main/guard when the feature is on. See [hotpath docs](https://hotpath.rs/).

Default `make` / CI builds **without** these features (no hotpath compile in the normal gate).

```bash
# Normal (CI)
cargo run -p my_feed

# Local performance report
cargo run -p my_feed --features hotpath
```

## Non-goals

- Replacing `serenade-profiler` with hotpath
- Pulling hotpath into `serenade-kernel` / `serenade-http`
- Requiring hotpath in org CI
