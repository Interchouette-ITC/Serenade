# Docker / compose targets for Serenade (integration Redis, etc.).
# Prefer Docker MCP or compose for ephemeral services in tests.

COMPOSE_FILE := $(ROOT)/docker/compose.yml
COMPOSE := docker compose -f $(COMPOSE_FILE) --project-directory $(ROOT)
DOCKER_BUILDKIT ?= 1
DOCKERFILE ?= $(ROOT)/docker/Dockerfile
HUB_IMAGE := interchouette/serenade-myfeed
GHCR_PERSONAL_IMAGE := ghcr.io/groussac/myfeed
GHCR_WORKER_IMAGE := ghcr.io/interchouette/myfeed
GHCR_ORG_IMAGE := ghcr.io/interchouette-itc/myfeed

.PHONY: docker-help docker-build-dev docker-push-dev-hub \
	docker-push-dev-ghcr-personal docker-push-dev-ghcr-itc \
	myfeed-up myfeed-down redis-up redis-down redis-test

docker-help:
	@echo "Docker targets:"
	@echo "  make docker-build-dev              build MyFeed :dev / :latest for Hub and GHCR"
	@echo "  make docker-push-dev-hub           push :dev / :latest to Docker Hub"
	@echo "  make docker-push-dev-ghcr-personal push :dev / :latest to personal GHCR"
	@echo "  make docker-push-dev-ghcr-itc      push :dev / :latest to Interchouette / ITC GHCR"
	@echo "  make myfeed-image alias for docker-build-dev"
	@echo "  make myfeed-up    build and start MyFeed on port 8080"
	@echo "  make myfeed-down  stop MyFeed"
	@echo "  make redis-up     start Redis (redis:7-alpine on 6379)"
	@echo "  make redis-down   stop Redis compose service"
	@echo "  make redis-test   cargo test -p serenade-cache --features redis"

docker-build-dev:
	DOCKER_BUILDKIT=$(DOCKER_BUILDKIT) docker build --pull --network=host \
		-f $(DOCKERFILE) -t $(HUB_IMAGE):dev $(ROOT)
	docker tag $(HUB_IMAGE):dev $(HUB_IMAGE):latest
	docker tag $(HUB_IMAGE):dev $(GHCR_PERSONAL_IMAGE):dev
	docker tag $(HUB_IMAGE):dev $(GHCR_PERSONAL_IMAGE):latest
	docker tag $(HUB_IMAGE):dev $(GHCR_WORKER_IMAGE):dev
	docker tag $(HUB_IMAGE):dev $(GHCR_WORKER_IMAGE):latest
	docker tag $(HUB_IMAGE):dev $(GHCR_ORG_IMAGE):dev
	docker tag $(HUB_IMAGE):dev $(GHCR_ORG_IMAGE):latest

docker-push-dev-hub:
	docker push $(HUB_IMAGE):dev
	docker push $(HUB_IMAGE):latest

docker-push-dev-ghcr-personal:
	docker push $(GHCR_PERSONAL_IMAGE):dev
	docker push $(GHCR_PERSONAL_IMAGE):latest

docker-push-dev-ghcr-itc:
	docker push $(GHCR_WORKER_IMAGE):dev
	docker push $(GHCR_WORKER_IMAGE):latest
	docker push $(GHCR_ORG_IMAGE):dev
	docker push $(GHCR_ORG_IMAGE):latest

myfeed-image: docker-build-dev

myfeed-up:
	$(COMPOSE) up -d --build myfeed

myfeed-down:
	$(COMPOSE) stop myfeed

redis-up:
	$(COMPOSE) up -d redis

redis-down:
	$(COMPOSE) down

redis-test: redis-up
	cd $(ROOT) && REDIS_URL=$${REDIS_URL:-redis://127.0.0.1:6379/0} \
		$(CARGO) test -p serenade-cache --features redis
