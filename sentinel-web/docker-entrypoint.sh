#!/bin/sh
# Prepare les certs TLS avant le demarrage de nginx.
# Priorite :
#   1. Cert Let's Encrypt monte sur /etc/letsencrypt/live/$WEB_DOMAIN/   (prod)
#   2. Cert self-signed genere localement                                 (dev)
#
# Place dans /docker-entrypoint.d/ pour etre execute par l'image nginx:alpine
# avant le lancement de `nginx -g "daemon off;"`.

set -eu

DOMAIN="${WEB_DOMAIN:-localhost}"
LE_DIR="/etc/letsencrypt/live/${DOMAIN}"
# nginx tourne en UID 101 (image unprivileged) et ne peut pas ecrire sous
# /etc/nginx : on place les certs generes/symlinks dans /tmp (inscriptible).
SELFSIGNED_DIR="/tmp/nginx-self-signed"
NGINX_CERT_DIR="/tmp/nginx-certs"

mkdir -p "${NGINX_CERT_DIR}"

if [ -f "${LE_DIR}/fullchain.pem" ] && [ -f "${LE_DIR}/privkey.pem" ]; then
    echo "[tls-init] Using Let's Encrypt cert for ${DOMAIN}"
    ln -sfn "${LE_DIR}/fullchain.pem" "${NGINX_CERT_DIR}/fullchain.pem"
    ln -sfn "${LE_DIR}/privkey.pem"   "${NGINX_CERT_DIR}/privkey.pem"
else
    echo "[tls-init] No Let's Encrypt cert at ${LE_DIR}. Falling back to self-signed."
    mkdir -p "${SELFSIGNED_DIR}"
    if [ ! -f "${SELFSIGNED_DIR}/fullchain.pem" ] || [ ! -f "${SELFSIGNED_DIR}/privkey.pem" ]; then
        echo "[tls-init] Generating self-signed cert (valid 30 days) for CN=${DOMAIN}..."
        openssl req -x509 -nodes -newkey rsa:2048 -days 30 \
            -keyout "${SELFSIGNED_DIR}/privkey.pem" \
            -out    "${SELFSIGNED_DIR}/fullchain.pem" \
            -subj "/CN=${DOMAIN}" \
            -addext "subjectAltName=DNS:${DOMAIN},DNS:localhost,IP:127.0.0.1" \
            >/dev/null 2>&1
        chmod 600 "${SELFSIGNED_DIR}/privkey.pem"
    fi
    ln -sfn "${SELFSIGNED_DIR}/fullchain.pem" "${NGINX_CERT_DIR}/fullchain.pem"
    ln -sfn "${SELFSIGNED_DIR}/privkey.pem"   "${NGINX_CERT_DIR}/privkey.pem"
    echo "[tls-init] WARNING: using self-signed cert. Browser will show a warning."
fi

# Prepare le webroot pour les challenges ACME HTTP-01 (certbot --webroot).
# nginx ne fait que LIRE ce repertoire ; c'est le sidecar certbot (root) qui y
# ecrit les challenges. En non-root, le mkdir peut echouer sur un volume
# root-only : on le rend tolerant (|| true) pour ne pas bloquer le demarrage.
mkdir -p /var/www/certbot/.well-known/acme-challenge 2>/dev/null || true
