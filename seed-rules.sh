#!/bin/bash
# ============================================
# DiscordSentinel - Seed des regles par defaut
# Insere les regles de moderation pour un serveur Discord.
#
# Usage:
#   bash seed-rules.sh                              # utilise le guild_id du .env
#   bash seed-rules.sh 1486472782303985866          # guild_id en argument
#   DATABASE_URL=postgres://... bash seed-rules.sh  # BDD distante
# ============================================

set -euo pipefail

GREEN='\033[0;32m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m'

# Charger .env si present
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
if [ -f "$SCRIPT_DIR/.env" ]; then
  set -a
  source "$SCRIPT_DIR/.env"
  set +a
fi

# Guild ID : argument ou .env
GUILD_ID="${1:-${VOICE_GUILD_ID:-}}"
if [ -z "$GUILD_ID" ]; then
  echo -e "${RED}Usage: bash seed-rules.sh <guild_id>${NC}"
  echo "  ou definir VOICE_GUILD_ID dans .env"
  exit 1
fi

# Database URL
DB_URL="${DATABASE_URL:-postgres://sentinel:sentinel_secret@localhost:5432/discord_sentinel}"

echo -e "${CYAN}================================================${NC}"
echo -e "${CYAN}  DiscordSentinel - Seed regles de moderation${NC}"
echo -e "${CYAN}  Guild: $GUILD_ID${NC}"
echo -e "${CYAN}================================================${NC}"
echo ""

# Regles par defaut :
#   flag_type         | weight | warn | delete | mute | ban
#   ------------------|--------|------|--------|------|----
#   spam              | 1.0    | 2.0  | 4.0    | 6.0  | 9.0
#   insult            | 1.5    | 2.0  | 3.0    | 5.0  | 8.0
#   link              | 0.5    | 3.0  | 5.0    | 7.0  | 9.0
#   phishing          | 3.0    | 1.0  | 2.0    | 3.0  | 5.0
#   nsfw              | 2.0    | 1.0  | 2.0    | 4.0  | 7.0
#   illicit           | 3.0    | 1.0  | 2.0    | 3.0  | 5.0
#   anger             | 0.8    | 3.0  | 5.0    | 7.0  | 9.0
#   rage              | 1.5    | 2.0  | 3.0    | 5.0  | 7.0
#   threat            | 2.5    | 1.0  | 2.0    | 3.0  | 5.0
#   harassment        | 2.0    | 1.0  | 2.0    | 4.0  | 6.0

SQL=$(cat <<'EOSQL'
INSERT INTO rules (id, guild_id, flag_type, weight, threshold_warn, threshold_delete, threshold_mute, threshold_ban, enabled)
VALUES
  (gen_random_uuid(), '$GUILD_ID', 'spam',       1.0, 2.0, 4.0, 6.0, 9.0, true),
  (gen_random_uuid(), '$GUILD_ID', 'insult',     1.5, 2.0, 3.0, 5.0, 8.0, true),
  (gen_random_uuid(), '$GUILD_ID', 'link',       0.5, 3.0, 5.0, 7.0, 9.0, true),
  (gen_random_uuid(), '$GUILD_ID', 'phishing',   3.0, 1.0, 2.0, 3.0, 5.0, true),
  (gen_random_uuid(), '$GUILD_ID', 'nsfw',       2.0, 1.0, 2.0, 4.0, 7.0, true),
  (gen_random_uuid(), '$GUILD_ID', 'illicit',    3.0, 1.0, 2.0, 3.0, 5.0, true),
  (gen_random_uuid(), '$GUILD_ID', 'anger',      0.8, 3.0, 5.0, 7.0, 9.0, true),
  (gen_random_uuid(), '$GUILD_ID', 'rage',       1.5, 2.0, 3.0, 5.0, 7.0, true),
  (gen_random_uuid(), '$GUILD_ID', 'threat',     2.5, 1.0, 2.0, 3.0, 5.0, true),
  (gen_random_uuid(), '$GUILD_ID', 'harassment', 2.0, 1.0, 2.0, 4.0, 6.0, true)
ON CONFLICT (guild_id, flag_type) DO UPDATE SET
  weight = EXCLUDED.weight,
  threshold_warn = EXCLUDED.threshold_warn,
  threshold_delete = EXCLUDED.threshold_delete,
  threshold_mute = EXCLUDED.threshold_mute,
  threshold_ban = EXCLUDED.threshold_ban,
  updated_at = NOW();
EOSQL
)

# Remplacer $GUILD_ID dans le SQL
SQL="${SQL//\$GUILD_ID/$GUILD_ID}"

echo "Insertion de 10 regles..."
echo ""

# Essayer via docker d'abord, sinon psql local
if docker ps --format '{{.Names}}' 2>/dev/null | grep -q "sentinel-postgres"; then
  echo -e "${CYAN}Via Docker (sentinel-postgres)...${NC}"
  echo "$SQL" | docker exec -i sentinel-postgres psql -U sentinel -d discord_sentinel
elif command -v psql &>/dev/null; then
  echo -e "${CYAN}Via psql local...${NC}"
  echo "$SQL" | psql "$DB_URL"
else
  echo -e "${RED}Ni Docker (sentinel-postgres) ni psql disponible.${NC}"
  echo "Copie ce SQL et execute-le manuellement :"
  echo ""
  echo "$SQL"
  exit 1
fi

echo ""
echo -e "${GREEN}10 regles de moderation inserees pour guild $GUILD_ID${NC}"
echo ""
echo "  spam, insult, link, phishing, nsfw,"
echo "  illicit, anger, rage, threat, harassment"
echo ""
echo -e "${CYAN}Modifiez les seuils depuis l'app bureau (page Regles).${NC}"
