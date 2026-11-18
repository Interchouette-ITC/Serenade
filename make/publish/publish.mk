# crates.io packaging and publication. Logic: make/publish/publish-crates.sh
# (order, version drift, skip-if-published, HTTP 429 waits, index propagation).
# Uploads run only from .github/workflows/release.yml: publish-crates.sh publish.

.PHONY: publish-check publish-test

## Packages and verifies every publishable crate in publish order (no upload).
## Local uncommitted trees: make publish-check PUBLISH_CHECK_ARGS=--allow-dirty
publish-check:
	CARGO="$(CARGO)" $(ROOT)/make/publish/publish-crates.sh check $(PUBLISH_CHECK_ARGS)

## Publisher tests against mocked cargo/curl/sleep (no network).
publish-test:
	$(ROOT)/make/publish/tests/publish-crates.test.sh
