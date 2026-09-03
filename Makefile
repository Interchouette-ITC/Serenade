# Serenade developer targets

SHELL := /bin/bash
ROOT := $(abspath .)
CARGO ?= cargo
CLIPPY_FLAGS := -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery
RUSTDOCFLAGS ?= -D warnings

.DEFAULT_GOAL := help

.PHONY: help check test lint format format-check doc doc-open clean ci

help:
	@echo "Serenade targets"
	@echo ""
	@echo "  make check        cargo check --workspace"
	@echo "  make test         cargo test --workspace"
	@echo "  make lint         fmt check + clippy (workspace)"
	@echo "  make doc          rustdoc for all crates (-D warnings)"
	@echo "  make doc-open     build docs and open in browser"
	@echo "  make format       cargo fmt"
	@echo "  make ci           lint + test + doc"
	@echo "  make clean        cargo clean"

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
