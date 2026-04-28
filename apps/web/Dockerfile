# syntax=docker/dockerfile:1.7
# Multi-stage : build Vite (Node) -> serve static via nginx (HTTP + HTTPS).
# Build context = racine du repo (cf. docker-compose.yml).

# ----- Stage 1 : build SPA -----
FROM node:22-alpine AS builder
WORKDIR /app

# Cache des deps : copie juste les manifests d'abord.
COPY apps/web/package.json apps/web/package-lock.json ./
RUN npm ci --no-audit --no-fund

# Sources
COPY apps/web/ ./

# URL de l'API injectee au build. Vide par defaut => fetch en relatif,
# donc les appels /api/* atterrissent sur le reverse-proxy nginx local.
# Override possible via `--build-arg VITE_API_URL=https://api.example.com`.
ARG VITE_API_URL=""
ENV VITE_API_URL=${VITE_API_URL}

RUN npm run build

# ----- Stage 2 : nginx runtime -----
FROM nginx:1.27-alpine AS runtime

# openssl : generation du cert self-signed en dev si aucun LE cert trouve.
RUN apk add --no-cache openssl

# Config nginx (HTTP -> HTTPS redirect + reverse-proxy TLS /api et /ws).
COPY apps/web/nginx.conf /etc/nginx/conf.d/default.conf

# Entrypoint : prepare les certs (self-signed fallback) puis lance nginx.
# `sed` strip des CR eventuels (si edite sous Windows) — sinon shebang casse.
COPY apps/web/docker-entrypoint.sh /docker-entrypoint.d/40-init-tls.sh
RUN sed -i 's/\r$//' /docker-entrypoint.d/40-init-tls.sh \
    && chmod +x /docker-entrypoint.d/40-init-tls.sh

# Assets statiques
COPY --from=builder /app/dist /usr/share/nginx/html

EXPOSE 80 443

# Healthcheck : endpoint HTTP /healthz expose par nginx sur le port 80
# (evite le probleme de cert self-signed non trustable en HTTPS).
HEALTHCHECK --interval=30s --timeout=3s --retries=3 --start-period=5s \
  CMD wget --spider -q http://localhost/healthz || exit 1
