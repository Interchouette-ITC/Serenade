# Shared Make variables (included by the root Makefile).

SHELL := /bin/bash
ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/..)
CARGO ?= cargo
# Lint/fmt on nightly; check/test/doc on stable (dual toolchain bar).
CARGO_NIGHTLY ?= cargo +nightly
CARGO_STABLE ?= cargo +stable
CLIPPY_FLAGS := -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery
RUSTDOCFLAGS ?= -D warnings

# Extra args for wrapper targets, e.g. `make serenade ARGS='recipe list'`
ARGS ?=
