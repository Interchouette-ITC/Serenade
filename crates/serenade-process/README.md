# serenade-process

Child process spawn, timeout, and exit/output capture (Symfony Process shaped).

`Process` builds the command (arguments, working directory, environment, timeout);
`Process::run` / `Process::must_run` return a `CompletedProcess` with status,
stdout, and stderr.

Part of [Serenade](https://github.com/Interchouette-ITC/Serenade); most applications
depend on the [`serenade-framework`](https://crates.io/crates/serenade-framework) facade.
API documentation: <https://docs.rs/serenade-process>.

Licensed under Apache-2.0.
