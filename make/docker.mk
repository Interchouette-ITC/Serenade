# Docker / compose targets for Serenade (integration Redis, etc.).
# Prefer Docker MCP or compose for ephemeral services in tests.

COMPOSE_FILE := $(ROOT)/docker/compose.yml
COMPOSE := docker compose -f $(COMPOSE_FILE) --project-directory $(ROOT)

.PHONY: docker-help redis-up redis-down redis-test

docker-help:
	@echo "Docker targets:"
	@echo "  make redis-up     start Redis (redis:7-alpine on 6379)"
	@echo "  make redis-down   stop Redis compose service"
	@echo "  make redis-test   cargo test -p serenade-cache --features redis"

redis-up:
	$(COMPOSE) up -d redis

redis-down:
	$(COMPOSE) down

redis-test: redis-up
	cd $(ROOT) && REDIS_URL=$${REDIS_URL:-redis://127.0.0.1:6379/0} \
		$(CARGO) test -p serenade-cache --features redis
