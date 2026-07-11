# syntax=docker/dockerfile:1.7
# Multi-stage : build Vite (Node) -> serve static via nginx (HTTP + HTTPS).
# Build context = racine du repo (cf. docker-compose.yml).

# ----- Stage 1 : build SPA -----
FROM node:22-alpine AS builder
WORKDIR /app

# Cache des deps : copie juste les manifests d'abord.
COPY sentinel-web/package.json sentinel-web/package-lock.json ./
RUN npm ci --no-audit --no-fund

# Sources
COPY sentinel-web/ ./

# URL de l'API injectee au build. Vide par defaut => fetch en relatif,
# donc les appels /api/* atterrissent sur le reverse-proxy nginx local.
# Override possible via `--build-arg VITE_API_URL=https://api.example.com`.
ARG VITE_API_URL=""
ENV VITE_API_URL=${VITE_API_URL}

RUN npm run build

# ----- Stage 2 : nginx runtime (NON-ROOT) -----
# Image unprivileged : le master ET les workers tournent en UID 101 (nginx),
# jamais en root. Ecoute par defaut sur des ports > 1024 (8080/8443) — le
# mapping host 80->8080 / 443->8443 est fait par docker-compose.
FROM nginxinc/nginx-unprivileged:1.27-alpine AS runtime

# apk + COPY/chmod exigent root au BUILD ; on redescend en 101 avant le CMD.
USER root

# openssl : generation du cert self-signed en dev si aucun LE cert trouve.
RUN apk add --no-cache openssl

# Config nginx (HTTP -> HTTPS redirect + reverse-proxy TLS /api et /ws).
COPY sentinel-web/nginx.conf /etc/nginx/conf.d/default.conf
# Headers de securite partages (inclus dans server + locations, cf. piege
# add_header documente dans le fichier).
COPY sentinel-web/security-headers.inc /etc/nginx/snippets/security-headers.inc

# Entrypoint : prepare les certs (self-signed fallback) puis lance nginx.
# `sed` strip des CR eventuels (si edite sous Windows) — sinon shebang casse.
COPY sentinel-web/docker-entrypoint.sh /docker-entrypoint.d/40-init-tls.sh
RUN sed -i 's/\r$//' /docker-entrypoint.d/40-init-tls.sh \
    && chmod +x /docker-entrypoint.d/40-init-tls.sh

# Assets statiques
COPY --from=builder /app/dist /usr/share/nginx/html

# Retour a l'utilisateur non-privilegie fourni par l'image (UID 101) pour
# l'execution : plus aucun process nginx ne tourne en root au runtime.
USER 101

EXPOSE 8080 8443

# Healthcheck : endpoint HTTP /healthz expose par nginx sur le port 8080
# (evite le probleme de cert self-signed non trustable en HTTPS).
HEALTHCHECK --interval=30s --timeout=3s --retries=3 --start-period=5s \
  CMD wget --spider -q http://localhost:8080/healthz || exit 1
