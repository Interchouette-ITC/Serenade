# CI / quality gates.

.PHONY: check test lint format format-check doc doc-open clean ci

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

ci: lint test doc

clean:
	cd $(ROOT) && $(CARGO) clean
