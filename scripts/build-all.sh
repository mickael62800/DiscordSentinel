#!/bin/bash
# ============================================
# DiscordSentinel - Build sequentiel
# Construit chaque image Docker une par une.
# Si un service echoue, les autres continuent.
#
# Usage:
#   bash build-all.sh          # build uniquement
#   bash build-all.sh --up     # build + lancement des conteneurs
#   bash build-all.sh --no-cache  # build sans cache Docker
# ============================================

set -o pipefail

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

FAILED=()
SUCCESS=()
TOTAL=0
CURRENT=0
START_UP=false
BUILD_ARGS=""

# Parse des arguments
for arg in "$@"; do
  case "$arg" in
    --up) START_UP=true ;;
    --no-cache) BUILD_ARGS="--no-cache" ;;
  esac
done

# Verification que Docker tourne
if ! docker info &>/dev/null; then
  echo -e "${RED}Docker n'est pas lance. Demarre Docker Desktop et reessaie.${NC}"
  exit 1
fi

# Liste de tous les services a builder (ordre: bots, workers, gateway, api)
BOTS=(automod-bot moderation-bot security-bot ticket-bot image-bot voice-bot progression-bot audit-bot community-bot roles-bot coude-bot blackjack-bot cleanup-bot game-bot welcome-bot)
WORKERS=(moderation-worker analytics-worker monitoring-worker cache-worker cleanup-worker coude-worker ai-worker appeal-sla-worker audit-cache-worker blackjack-cleanup-worker discord-audit-sync-worker export-worker temp-roles-worker)
SERVICES=("${BOTS[@]}" "${WORKERS[@]}" gateway api)
TOTAL=${#SERVICES[@]}

build_service() {
  local svc="$1"
  CURRENT=$((CURRENT + 1))
  local svc_start=$SECONDS

  echo ""
  echo -e "${CYAN}========================================${NC}"
  echo -e "${CYAN}  [$CURRENT/$TOTAL] BUILD: $svc${NC}"
  echo -e "${CYAN}========================================${NC}"

  if docker compose build $BUILD_ARGS "$svc" 2>&1; then
    local duration=$((SECONDS - svc_start))
    echo -e "${GREEN}[OK] $svc (${duration}s)${NC}"
    SUCCESS+=("$svc")
  else
    local duration=$((SECONDS - svc_start))
    echo -e "${RED}[FAIL] $svc (${duration}s)${NC}"
    FAILED+=("$svc")
  fi
}

# Demarrage
GLOBAL_START=$SECONDS

echo ""
echo -e "${CYAN}================================================${NC}"
echo -e "${CYAN}  DiscordSentinel - Build sequentiel ($TOTAL services)${NC}"
echo -e "${CYAN}================================================${NC}"

for svc in "${SERVICES[@]}"; do
  build_service "$svc"
done

# Resume
GLOBAL_DURATION=$((SECONDS - GLOBAL_START))
MINUTES=$((GLOBAL_DURATION / 60))
SECS=$((GLOBAL_DURATION % 60))

echo ""
echo -e "${CYAN}========================================${NC}"
echo -e "${CYAN}  RESUME  (${MINUTES}m ${SECS}s)${NC}"
echo -e "${CYAN}========================================${NC}"
echo -e "${GREEN}OK (${#SUCCESS[@]}):${NC} ${SUCCESS[*]}"

if [ ${#FAILED[@]} -gt 0 ]; then
  echo -e "${RED}ECHEC (${#FAILED[@]}):${NC} ${FAILED[*]}"
  echo ""
  echo -e "${YELLOW}Relance uniquement les echecs avec :${NC}"
  echo -e "  docker compose build ${FAILED[*]}"
  exit 1
else
  echo -e "${GREEN}Tout est builde avec succes !${NC}"
fi

# Lancement si --up
if [ "$START_UP" = true ]; then
  echo ""
  echo -e "${CYAN}Lancement des conteneurs...${NC}"
  docker compose up -d
  echo ""
  docker compose ps
fi
