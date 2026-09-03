# Shared Make variables (included by the root Makefile).

SHELL := /bin/bash
ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/..)
CARGO ?= cargo
CLIPPY_FLAGS := -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery
RUSTDOCFLAGS ?= -D warnings

# Extra args for wrapper targets, e.g. `make serenade ARGS='recipe list'`
ARGS ?=
