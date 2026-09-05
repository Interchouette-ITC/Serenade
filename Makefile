# Serenade developer targets
#
# Layout (GNU Make includes):
#   make/common.mk  - shared variables
#   make/ci.mk      - lint / test / doc / ci
#   make/cli.mk     - serenade / tui / console aliases
#   make/docker.mk  - container targets (placeholder until needed)

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
	@echo "  make coverage     cargo llvm-cov → coverage/lcov.info"
	@echo "  make lint         fmt check + clippy (workspace)"
	@echo "  make doc          rustdoc → docs/api-rust/ (-D warnings)"
	@echo "  make doc-open     build docs and open docs/api-rust/index.html"
	@echo "  make format       cargo fmt"
	@echo "  make audit        cargo audit"
	@echo "  make deny         cargo deny check"
	@echo "  make ci           lint + test + doc"
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
	@echo ""
	@echo "Docker (make/docker.mk):"
	@echo "  make docker-help"
