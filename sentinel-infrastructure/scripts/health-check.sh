#!/bin/bash
# ============================================
# DiscordSentinel - Health Check
# Verifie que tous les services tournent correctement.
#
# Usage:
#   bash health-check.sh           # verification complete
#   bash health-check.sh --watch   # surveillance continue (toutes les 30s)
# ============================================

set -o pipefail

cd "$(dirname "$0")/../.."
export COMPOSE_FILE=sentinel-infrastructure/docker/docker-compose.yml

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

WATCH=false
for arg in "$@"; do
  [ "$arg" = "--watch" ] && WATCH=true
done

# Verification que Docker tourne
if ! docker info &>/dev/null; then
  echo -e "${RED}Docker n'est pas lance.${NC}"
  exit 1
fi

run_check() {
  local OK=0
  local WARN=0
  local FAIL=0
  local ISSUES=()

  echo ""
  echo -e "${CYAN}================================================${NC}"
  echo -e "${CYAN}  DiscordSentinel - Health Check${NC}"
  echo -e "${CYAN}  $(date '+%Y-%m-%d %H:%M:%S')${NC}"
  echo -e "${CYAN}================================================${NC}"

  # ── 1. Conteneurs ──
  echo ""
  echo -e "${CYAN}── Conteneurs ──${NC}"

  local containers
  containers=$(docker compose ps -a --format '{{.Name}}|{{.Status}}|{{.Service}}' 2>/dev/null | grep -v "level=warning")

  if [ -z "$containers" ]; then
    echo -e "${RED}  Aucun conteneur trouve. Lance 'bash start-all.sh' d'abord.${NC}"
    return 1
  fi

  while IFS='|' read -r name status service; do
    local icon=""
    if echo "$status" | grep -qi "up.*healthy"; then
      icon="${GREEN}[OK]${NC}"
      OK=$((OK + 1))
    elif echo "$status" | grep -qi "up"; then
      icon="${GREEN}[UP]${NC}"
      OK=$((OK + 1))
    elif echo "$status" | grep -qi "restarting"; then
      icon="${RED}[RESTART]${NC}"
      FAIL=$((FAIL + 1))
      ISSUES+=("$name est en boucle de redemarrage")
    elif echo "$status" | grep -qi "exited"; then
      icon="${YELLOW}[STOP]${NC}"
      WARN=$((WARN + 1))
      ISSUES+=("$name est arrete")
    else
      icon="${RED}[???]${NC}"
      FAIL=$((FAIL + 1))
      ISSUES+=("$name statut inconnu: $status")
    fi
    printf "  %-40s %b\n" "$name" "$icon  $status"
  done <<< "$containers"

  # ── 2. Logs d'erreurs recentes (derniere minute) ──
  echo ""
  echo -e "${CYAN}── Erreurs recentes (60s) ──${NC}"

  local error_services=()
  local services
  services=$(docker compose ps -a --format '{{.Service}}' 2>/dev/null | grep -v "level=warning")

  for svc in $services; do
    local errors
    errors=$(docker compose logs "$svc" --since 60s 2>/dev/null | grep -ci "error\|panic\|fatal" || true)
    if [ "$errors" -gt 0 ]; then
      echo -e "  ${RED}$svc : $errors erreur(s)${NC}"
      error_services+=("$svc")
      WARN=$((WARN + 1))
    fi
  done

  if [ ${#error_services[@]} -eq 0 ]; then
    echo -e "  ${GREEN}Aucune erreur detectee${NC}"
  fi

  # ── 3. Connectivite API ──
  echo ""
  echo -e "${CYAN}── Connectivite ──${NC}"

  # API
  if docker compose ps api --format '{{.Status}}' 2>/dev/null | grep -qi "up"; then
    local api_response
    api_response=$(docker compose exec -T api sh -c 'curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/health 2>/dev/null' || echo "000")
    if [ "$api_response" = "200" ]; then
      echo -e "  API (health)        ${GREEN}[OK] HTTP 200${NC}"
    elif [ "$api_response" = "000" ]; then
      echo -e "  API (health)        ${YELLOW}[N/A] pas de /health ou curl absent${NC}"
    else
      echo -e "  API (health)        ${RED}[FAIL] HTTP $api_response${NC}"
      ISSUES+=("API retourne HTTP $api_response")
    fi
  fi

  # Postgres
  if docker compose ps postgres --format '{{.Status}}' 2>/dev/null | grep -qi "healthy"; then
    echo -e "  PostgreSQL          ${GREEN}[OK] healthy${NC}"
  else
    echo -e "  PostgreSQL          ${RED}[FAIL] pas healthy${NC}"
    ISSUES+=("PostgreSQL n'est pas healthy")
    FAIL=$((FAIL + 1))
  fi

  # Redis
  if docker compose ps redis --format '{{.Status}}' 2>/dev/null | grep -qi "healthy"; then
    echo -e "  Redis               ${GREEN}[OK] healthy${NC}"
  else
    echo -e "  Redis               ${RED}[FAIL] pas healthy${NC}"
    ISSUES+=("Redis n'est pas healthy")
    FAIL=$((FAIL + 1))
  fi

  # ── 4. Ressources Docker ──
  echo ""
  echo -e "${CYAN}── Ressources ──${NC}"
  docker stats --no-stream --format "  {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}" 2>/dev/null | sort | head -25

  # ── Resume ──
  echo ""
  echo -e "${CYAN}========================================${NC}"
  if [ $FAIL -eq 0 ] && [ $WARN -eq 0 ]; then
    echo -e "${GREEN}  Tout est OK ($OK services)${NC}"
  elif [ $FAIL -eq 0 ]; then
    echo -e "${YELLOW}  $OK OK / $WARN avertissement(s)${NC}"
  else
    echo -e "${RED}  $OK OK / $WARN avertissement(s) / $FAIL erreur(s)${NC}"
  fi

  if [ ${#ISSUES[@]} -gt 0 ]; then
    echo ""
    echo -e "${YELLOW}  Problemes detectes :${NC}"
    for issue in "${ISSUES[@]}"; do
      echo -e "  ${RED}  - $issue${NC}"
    done
  fi
  echo -e "${CYAN}========================================${NC}"
}

if [ "$WATCH" = true ]; then
  echo -e "${CYAN}Mode surveillance (Ctrl+C pour arreter)${NC}"
  while true; do
    clear
    run_check
    echo ""
    echo -e "${YELLOW}Prochaine verification dans 30s...${NC}"
    sleep 30
  done
else
  run_check
fi
