# CI / quality gates.

.PHONY: check test lint format format-check sweep doc doc-open doc-clean clean ci audit deny \
	coverage coverage-summary coverage-html tarpaulin machete outdated fuzz fuzz-build geiger

# Keep in sync with codecov.yml ignore paths and CI coverage.
COVERAGE_FEATURES := --features serenade-cache/redis,serenade-lock/redis,serenade-messenger/redis,serenade-rate-limiter/redis,serenade-serializer/toon,serenade-scheduler/messenger,serenade-security/oauth,serenade-security/ldap,serenade-mailer/esp,serenade-search/http
COVERAGE_IGNORE := examples/|crates/serenade-cli/|serenade-console/src/(interactive\.rs|commands/)|/serenade-contracts/src/tests\.rs

check:
	cd $(ROOT) && $(CARGO_STABLE) check --workspace --locked

test:
	cd $(ROOT) && $(CARGO_STABLE) test --workspace --locked

## Requires `cargo install cargo-llvm-cov`. Writes `coverage/lcov.info` (CI / Codecov).
## Uses the stable toolchain so llvm-cov finds instrumented objects.
coverage:
	cd $(ROOT) && mkdir -p coverage && $(CARGO_STABLE) llvm-cov --workspace --locked --lcov \
		$(COVERAGE_FEATURES) \
		--ignore-filename-regex '$(COVERAGE_IGNORE)' \
		--output-path coverage/lcov.info

## Terminal summary only (fast local check). Same features / ignores as `coverage`.
coverage-summary:
	cd $(ROOT) && $(CARGO_STABLE) llvm-cov --workspace --locked --summary-only \
		$(COVERAGE_FEATURES) \
		--ignore-filename-regex '$(COVERAGE_IGNORE)'

## HTML report → `coverage/html/` (open coverage/html/index.html).
coverage-html:
	cd $(ROOT) && mkdir -p coverage && $(CARGO_STABLE) llvm-cov --workspace --locked --html \
		$(COVERAGE_FEATURES) \
		--ignore-filename-regex '$(COVERAGE_IGNORE)' \
		--output-dir coverage/html

## Alternate local coverage via tarpaulin (not used by CI; Codecov stays on llvm-cov).
## Requires `cargo install cargo-tarpaulin`. Writes under `coverage/tarpaulin/`.
tarpaulin:
	cd $(ROOT) && mkdir -p coverage/tarpaulin && $(CARGO_STABLE) tarpaulin --workspace --locked \
		$(COVERAGE_FEATURES) \
		--exclude-files 'examples/*' 'crates/serenade-cli/*' \
		'serenade-console/src/interactive.rs' 'serenade-console/src/commands/*' \
		'crates/serenade-contracts/src/tests.rs' \
		--out Html --out Xml --output-dir coverage/tarpaulin

## Unused workspace dependencies. Requires `cargo install cargo-machete`.
machete:
	cd $(ROOT) && $(CARGO_STABLE) machete

## Outdated crates report. Requires `cargo install cargo-outdated`.
outdated:
	cd $(ROOT) && $(CARGO) outdated --workspace

## Default fuzz target: serenade_form::parse_urlencoded.
## Requires nightly + `cargo install cargo-fuzz`. Override: FUZZ_TARGET=… FUZZ_TIME=…
FUZZ_TARGET ?= parse-urlencoded
FUZZ_TIME ?= 10

fuzz-build:
	cd $(ROOT) && cargo +nightly fuzz build

fuzz:
	cd $(ROOT) && cargo +nightly fuzz run $(FUZZ_TARGET) -- -max_total_time=$(FUZZ_TIME)

## Unsafe Rust surface for the demo-html demo tree (framework consumer, MyFeed).
## Requires `cargo-geiger` and `jq`. Fails when red packages exceed
## `.geiger-budget.toml` (`max_red_packages`). Fresh target dir avoids stale `.d`
## files from rust-cache breaking the scan.
GEIGER_MANIFEST ?= $(ROOT)/examples/demo-html/Cargo.toml
geiger:
	@mkdir -p $(ROOT)/target/geiger
	@rm -rf $(ROOT)/target/geiger-scan
	cd $(ROOT) && $(CARGO_STABLE) fetch --locked
	cd $(ROOT) && CARGO_TARGET_DIR="$(ROOT)/target/geiger-scan" $(CARGO_STABLE) geiger \
		--locked --manifest-path "$(GEIGER_MANIFEST)" --output-format Json --quiet \
		>$(ROOT)/target/geiger/report.json
	@command -v jq >/dev/null || { echo "jq is required for make geiger" >&2; exit 2; }
	@budget=$$(awk -F= '/^[[:space:]]*max_red_packages[[:space:]]*=/{gsub(/[[:space:]]/,"",$$2); print $$2; exit}' $(ROOT)/.geiger-budget.toml); \
	test -n "$$budget" || { echo ".geiger-budget.toml: missing max_red_packages" >&2; exit 2; }; \
	report=$(ROOT)/target/geiger/report.json; \
	total=$$(jq '.packages | length' "$$report"); \
	red=$$(jq '[.packages[] | (.unsafety.used // {}) as $$u | (($$u.functions.unsafe_ // 0) + ($$u.exprs.unsafe_ // 0) + ($$u.item_impls.unsafe_ // 0) + ($$u.item_traits.unsafe_ // 0) + ($$u.methods.unsafe_ // 0)) as $$n | select($$n > 0)] | length' "$$report"); \
	{ \
	  echo "## cargo-geiger"; echo; \
	  echo "- Packages scanned: **$$total**"; \
	  echo "- Red (used unsafe > 0): **$$red**"; \
	  echo "- Budget (\`max_red_packages\`): **$$budget**"; echo; \
	  if [ "$$red" -gt "$$budget" ]; then echo "**FAIL:** red count $$red exceeds budget $$budget."; \
	  else echo "OK: red count $$red within budget $$budget."; fi; \
	} | tee $(ROOT)/target/geiger/summary.md; \
	if [ "$$red" -gt "$$budget" ]; then echo "geiger budget exceeded: $$red red packages > $$budget" >&2; exit 1; fi

format:
	cd $(ROOT) && $(CARGO_NIGHTLY) fmt

format-check:
	cd $(ROOT) && $(CARGO_NIGHTLY) fmt --check

## Em/en dash + forbidden clippy allows. Invoked from `make lint`.
sweep:
	@$(ROOT)/make/sweep.sh

lint: format-check sweep
	cd $(ROOT) && $(CARGO_NIGHTLY) clippy --workspace --all-targets --locked -- $(CLIPPY_FLAGS)

## rustdoc → `docs/api-rust/` (GitHub Pages publish folder).
DOC_OUT ?= $(ROOT)/target/doc
DOC_CRATE ?= serenade_kernel

doc:
	cd $(ROOT) && RUSTDOCFLAGS="$(RUSTDOCFLAGS)" $(CARGO_STABLE) doc --workspace --no-deps --locked --features serenade-cache/redis,serenade-lock/redis,serenade-messenger/redis,serenade-rate-limiter/redis,serenade-serializer/toon,serenade-scheduler/messenger,serenade-security/oauth,serenade-security/ldap,serenade-mailer/esp,serenade-search/http
	@test -d "$(DOC_OUT)" || (echo "missing $(DOC_OUT)"; exit 1)
	@rm -rf $(ROOT)/docs/api-rust
	@mkdir -p $(ROOT)/docs/api-rust
	@cp -a "$(DOC_OUT)/." $(ROOT)/docs/api-rust/
	@printf '%s\n' \
		'<!DOCTYPE html>' \
		'<html lang="en">' \
		'<head>' \
		'<meta charset="utf-8">' \
		'<meta http-equiv="refresh" content="0; url=$(DOC_CRATE)/index.html">' \
		'<title>Serenade - Rust API docs</title>' \
		'<link rel="canonical" href="$(DOC_CRATE)/index.html">' \
		'<script>location.replace("$(DOC_CRATE)/index.html");</script>' \
		'</head>' \
		'<body><p><a href="$(DOC_CRATE)/index.html">Serenade API documentation</a></p></body>' \
		'</html>' \
		> $(ROOT)/docs/api-rust/index.html
	@touch $(ROOT)/docs/api-rust/.nojekyll
	@printf '%s\n' \
		'# Rust API documentation (rustdoc)' \
		'' \
		'Open [`$(DOC_CRATE)/index.html`]($(DOC_CRATE)/index.html) (root [`index.html`](index.html) redirects there).' \
		> $(ROOT)/docs/api-rust/README.md
	@echo "docs/api-rust/ updated - open docs/api-rust/index.html"

doc-open: doc
	@xdg-open $(ROOT)/docs/api-rust/index.html 2>/dev/null \
		|| open $(ROOT)/docs/api-rust/index.html 2>/dev/null \
		|| echo "Open docs/api-rust/index.html in a browser"

doc-clean:
	rm -rf $(ROOT)/docs/api-rust
	mkdir -p $(ROOT)/docs/api-rust
	@printf '%s\n' \
		'# Rust API documentation (rustdoc)' \
		'' \
		'Generated by `make doc`. Open [`index.html`](index.html) after building.' \
		> $(ROOT)/docs/api-rust/README.md

## Requires `cargo install cargo-audit`.
## RUSTSEC-2026-0258: actix-http 3.x pins h2 0.3; fix is only on h2 >= 0.4.16.
## Wasmer / serde_yaml stack advisories: same ignores as deny.toml.
## zip 3.x is a build-dep of utoipa-swagger-ui 9; no release yet accepts zip >=9.
audit:
	cd $(ROOT) && $(CARGO_STABLE) audit \
		--ignore RUSTSEC-2026-0258 \
		--ignore RUSTSEC-2026-0367 \
		--ignore RUSTSEC-2026-0368 \
		--ignore RUSTSEC-2026-0369

## Requires `cargo install cargo-deny`.
deny:
	cd $(ROOT) && $(CARGO_STABLE) deny check

ci: lint test doc audit deny machete

clean:
	cd $(ROOT) && $(CARGO_STABLE) clean
