#!/bin/sh
# Genere la configuration PUBLIQUE du site, lue par le front au demarrage.
#
# Pourquoi a l'execution et non au build Vite : ces valeurs changent sans que
# le code change. Passer par `ARG` obligerait a reconstruire toute l'image
# pour corriger un lien d'invitation Discord, alors qu'un simple redemarrage
# du conteneur suffit ici.
#
# Ce fichier est servi tel quel a n'importe quel visiteur : il ne doit
# contenir QUE des valeurs deja publiques. Aucune cle, aucun secret. Un
# identifiant de serveur Discord et un lien d'invitation en sont, ils
# figurent deja dans toute URL du serveur.
#
# Place dans /docker-entrypoint.d/ : l'image nginx execute ces scripts par
# ordre alphabetique avant de lancer nginx.

set -eu

CIBLE="/usr/share/nginx/html/site-config.json"

# `printf '%s'` et non `echo` : les valeurs viennent de l'environnement et
# pourraient contenir des sequences que `echo` interpreterait.
printf '{"guild_id":"%s","discord_invite":"%s"}\n' \
    "${PUBLIC_GUILD_ID:-}" \
    "${DISCORD_INVITE:-}" > "${CIBLE}"

if [ -z "${PUBLIC_GUILD_ID:-}" ]; then
    echo "[site-config] WARNING: PUBLIC_GUILD_ID absente — l'espace membre n'affichera aucune section"
else
    echo "[site-config] guilde publique = ${PUBLIC_GUILD_ID}"
fi
