# Docker / compose targets for Serenade (integration DB, etc.).
# Add targets here when compose recipes land; keep Hub/GHCR publish habits out of CI slices.

.PHONY: docker-help

docker-help:
	@echo "No Docker Make targets yet. Prefer Docker MCP or compose docs when added."
