# ============================================================================
# docker buildx bake — builds parallels.
# Lance toujours depuis la racine du repo :
#   docker buildx bake -f infrastructure/docker/docker-bake.hcl
#   docker buildx bake -f infrastructure/docker/docker-bake.hcl workers
#   docker buildx bake -f infrastructure/docker/docker-bake.hcl api gateway bot
#
# Surcharger le tag :
#   TAG=v1.2.3 docker buildx bake -f infrastructure/docker/docker-bake.hcl
# ============================================================================

variable "TAG" { default = "local" }

group "default" {
  targets = ["api", "gateway", "bot", "web", "workers"]
}

group "workers" {
  targets = [
    "worker-ai", "worker-analytics", "worker-appeal-sla", "worker-audit-cache",
    "worker-cache", "worker-cleanup",
    "worker-discord-audit-sync", "worker-export", "worker-moderation",
    "worker-monitoring", "worker-temp-roles",
  ]
}

target "_alpine-base" {
  context    = "."
  dockerfile = "infrastructure/docker/Dockerfile.rust-alpine"
}

target "_debian-base" {
  context    = "."
  dockerfile = "infrastructure/docker/Dockerfile.rust-debian"
}

target "api" {
  inherits = ["_debian-base"]
  args = {
    BIN_NAME       = "sentinel-api"
    MIGRATIONS_SRC = "sentinel-api/migrations"
  }
  tags = ["sentinel/api:${TAG}"]
}

target "gateway" {
  inherits = ["_alpine-base"]
  args     = { BIN_NAME = "sentinel-gateway" }
  tags     = ["sentinel/gateway:${TAG}"]
}

target "bot" {
  inherits = ["_alpine-base"]
  args = {
    BIN_NAME   = "sentinel-bot"
  }
  tags = ["sentinel/bot:${TAG}"]
}

target "web" {
  context    = "."
  dockerfile = "web/Dockerfile"
  tags       = ["sentinel/web:${TAG}"]
}

# Matrix : 13 workers en une seule cible parametrique (buildx bake >= 0.13).
target "worker" {
  inherits = ["_alpine-base"]
  matrix = {
    name = [
      "ai", "analytics", "appeal-sla", "audit-cache",
      "cache", "cleanup", "discord-audit-sync", "export",
      "moderation", "monitoring", "temp-roles",
    ]
  }
  name = "worker-${name}"
  args = { BIN_NAME = "sentinel-${name}-worker" }
  tags = ["sentinel/${name}-worker:${TAG}"]
}
