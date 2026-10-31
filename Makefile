# Serenade developer targets
#
# Layout (GNU Make includes):
#   make/common.mk  - shared variables
#   make/ci.mk      - lint / test / doc / ci
#   make/cli.mk     - serenade / tui / console aliases
#   make/docker.mk  - Redis compose and container targets

include make/common.mk
include make/ci.mk
include make/cli.mk
include make/docker.mk

.DEFAULT_GOAL := help

.PHONY: help

help:
	@echo "Serenade targets"
	@echo ""
	@echo "Quality (make/ci.mk):"
	@echo "  make check        cargo check --workspace"
	@echo "  make test         cargo test --workspace"
	@echo "  make coverage         cargo llvm-cov lcov → coverage/lcov.info (CI / Codecov)"
	@echo "  make coverage-summary cargo llvm-cov --summary-only"
	@echo "  make coverage-html    cargo llvm-cov HTML → coverage/html/"
	@echo "  make tarpaulin        cargo tarpaulin → coverage/tarpaulin/ (local alternate)"
	@echo "  make machete          cargo machete (unused deps)"
	@echo "  make outdated         cargo outdated --workspace"
	@echo "  make fuzz             cargo +nightly fuzz run $(FUZZ_TARGET) (FUZZ_TIME=$(FUZZ_TIME)s)"
	@echo "  make fuzz-build       cargo +nightly fuzz build"
	@echo "  make geiger           cargo geiger vs .geiger-budget.toml (CI job)"
	@echo "  make lint         nightly fmt + sweep + clippy (workspace)"
	@echo "  make sweep        em/en dash + forbidden clippy allows"
	@echo "  make doc          rustdoc → docs/api-rust/ (-D warnings, stable)"
	@echo "  make doc-open     build docs and open docs/api-rust/index.html"
	@echo "  make format       cargo +nightly fmt"
	@echo "  make audit        cargo audit"
	@echo "  make deny         cargo deny check"
	@echo "  make ci           lint + test + doc + audit + deny + machete"
	@echo "  make clean        cargo clean"
	@echo ""
	@echo "CLI / demo (make/cli.mk) - prefer these over cargo run -p ...:"
	@echo "  make serenade                 scaffolding CLI (pass ARGS=...)"
	@echo "  make serenade ARGS='--help'"
	@echo "  make serenade ARGS='new demo --path /tmp'"
	@echo "  make serenade ARGS='recipe list'"
	@echo "  make serenade ARGS='recipe apply security --no-cargo'"
	@echo "  make tui   / make serenade-tui   guided recipe picker (ARGS for flags)"
	@echo "  make tui ARGS='--no-cargo'"
	@echo "  make recipe-list"
	@echo "  make console ARGS='serenade:about'"
	@echo "  make console ARGS='--interactive'"
	@echo "  make demo"
	@echo "  make html                      run the default HTML-mode app"
	@echo "  make html-all                  run every registered HTML app"
	@echo "  make myfeed                    legacy alias for make html"
	@echo "  make spa                       build the default SPA app with Trunk"
	@echo "  make spa-all                   build every registered SPA app"
	@echo "  make ssg                       build the default SSG app"
	@echo "  make ssg-all                   build every registered SSG app"
	@echo "  make universal                 build and run the Universal demo (:8080)"
	@echo ""
	@echo "Docker (make/docker.mk):"
	@echo "  make docker-help"
	@echo "  make redis-up / redis-down / redis-test"
