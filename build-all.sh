#!/bin/bash
# Build séquentiel — chaque image est construite indépendamment.
# Si un service échoue, les autres continuent.

set -o pipefail

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

FAILED=()
SUCCESS=()

build_service() {
  local svc="$1"
  echo ""
  echo "========================================"
  echo "  BUILD: $svc"
  echo "========================================"
  if docker compose build "$svc" 2>&1; then
    echo -e "${GREEN}[OK] $svc${NC}"
    SUCCESS+=("$svc")
  else
    echo -e "${RED}[FAIL] $svc${NC}"
    FAILED+=("$svc")
  fi
}

# 1. Bots
for bot in automod-bot moderation-bot security-bot ticket-bot image-bot voice-bot progression-bot audit-bot community-bot roles-bot coude-bot; do
  build_service "$bot"
done

# 2. Workers
for worker in moderation-worker analytics-worker monitoring-worker cache-worker cleanup-worker coude-worker; do
  build_service "$worker"
done

# 3. Gateway
build_service "gateway"

# 4. API (le plus lourd, en dernier)
build_service "api"

# Résumé
echo ""
echo "========================================"
echo "  RESUME"
echo "========================================"
echo -e "${GREEN}OK (${#SUCCESS[@]}):${NC} ${SUCCESS[*]}"
if [ ${#FAILED[@]} -gt 0 ]; then
  echo -e "${RED}ECHEC (${#FAILED[@]}):${NC} ${FAILED[*]}"
  exit 1
else
  echo -e "${GREEN}Tout est buildé avec succès !${NC}"
fi
