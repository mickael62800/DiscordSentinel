#!/bin/sh
# Genere le snippet nginx qui injecte la cle d'API Nexus cote serveur.
#
# Pourquoi un fichier genere plutot qu'une valeur en dur dans nginx.conf :
# nginx.conf est copie tel quel dans l'image (pas de template envsubst), et on
# ne veut evidemment pas d'un secret commite. Le snippet est donc ecrit au
# demarrage du conteneur a partir de la variable d'environnement.
#
# Place dans /docker-entrypoint.d/ : l'image nginx:alpine execute ces scripts
# par ordre alphabetique avant de lancer nginx.

set -eu

SNIPPET_DIR="/etc/nginx/snippets"
SNIPPET="${SNIPPET_DIR}/nexus-auth.inc"

mkdir -p "${SNIPPET_DIR}"

if [ -n "${NEXUS_API_KEY:-}" ]; then
    printf 'proxy_set_header Authorization "Bearer %s";\n' "${NEXUS_API_KEY}" > "${SNIPPET}"
    chmod 600 "${SNIPPET}"
    echo "[nexus-key] Cle Nexus injectee dans le proxy /nexus-api/"
else
    # Fichier vide : la directive `include` de nginx.conf reste valide, la
    # requete part sans Authorization et nexus-api repond 401. Sans ce
    # fichier, nginx refuserait de demarrer (include introuvable).
    : > "${SNIPPET}"
    echo "[nexus-key] WARNING: NEXUS_API_KEY absente — /nexus-api/ repondra 401"
fi
