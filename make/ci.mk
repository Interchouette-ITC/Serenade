# CI / quality gates.

.PHONY: check test lint format format-check doc doc-open clean ci audit deny

check:
	cd $(ROOT) && $(CARGO) check --workspace

test:
	cd $(ROOT) && $(CARGO) test --workspace

format:
	cd $(ROOT) && $(CARGO) fmt

format-check:
	cd $(ROOT) && $(CARGO) fmt --check

lint: format-check
	cd $(ROOT) && $(CARGO) clippy --workspace --all-targets -- $(CLIPPY_FLAGS)

doc:
	cd $(ROOT) && RUSTDOCFLAGS="$(RUSTDOCFLAGS)" $(CARGO) doc --workspace --no-deps

doc-open: doc
	cd $(ROOT) && RUSTDOCFLAGS="$(RUSTDOCFLAGS)" $(CARGO) doc --workspace --no-deps --open

## Requires `cargo install cargo-audit`.
## RUSTSEC-2026-0258: actix-http 3.x pins h2 0.3; fix is only on h2 >= 0.4.16.
audit:
	cd $(ROOT) && $(CARGO) audit --ignore RUSTSEC-2026-0258

## Requires `cargo install cargo-deny`.
deny:
	cd $(ROOT) && $(CARGO) deny check

ci: lint test doc

clean:
	cd $(ROOT) && $(CARGO) clean
