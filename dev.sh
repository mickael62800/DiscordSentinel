#!/bin/bash
# ============================================
# DiscordSentinel - Dev Launcher
# Lance l'API, les bots et l'app desktop
# en parallele pour le developpement local
# ============================================

set -e

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
LOG_DIR="$ROOT_DIR/.logs"
mkdir -p "$LOG_DIR"

# Couleurs
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# PIDs des processus lances
PIDS=()

cleanup() {
    echo ""
    echo -e "${YELLOW}Arret de tous les services...${NC}"
    for pid in "${PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            kill "$pid" 2>/dev/null
        fi
    done
    wait 2>/dev/null
    echo -e "${GREEN}Tous les services sont arretes.${NC}"
    exit 0
}

trap cleanup SIGINT SIGTERM

# Charger .env si present
if [ -f "$ROOT_DIR/.env" ]; then
    echo -e "${CYAN}Chargement de .env...${NC}"
    set -a
    source "$ROOT_DIR/.env"
    set +a
fi

# ──────────────────────────────────────────────
# Verification des prerequis
# ──────────────────────────────────────────────

check_prereqs() {
    local missing=0

    if ! command -v cargo &>/dev/null; then
        echo -e "${RED}cargo non trouve. Installe Rust : https://rustup.rs${NC}"
        missing=1
    fi

    if ! command -v node &>/dev/null; then
        echo -e "${RED}node non trouve. Installe Node.js : https://nodejs.org${NC}"
        missing=1
    fi

    if [ "$missing" -eq 1 ]; then
        exit 1
    fi
}

# ──────────────────────────────────────────────
# Lancement des services
# ──────────────────────────────────────────────

start_service() {
    local name="$1"
    local dir="$2"
    local cmd="$3"
    local color="$4"
    local log_file="$LOG_DIR/$name.log"

    if [ ! -d "$dir" ]; then
        echo -e "${YELLOW}[SKIP] $name - dossier $dir introuvable${NC}"
        return
    fi

    echo -e "${color}[START] $name${NC} (logs: .logs/$name.log)"
    (cd "$dir" && $cmd > "$log_file" 2>&1) &
    PIDS+=($!)
}

# ──────────────────────────────────────────────
# Main
# ──────────────────────────────────────────────

echo ""
echo -e "${CYAN}================================================${NC}"
echo -e "${CYAN}   DiscordSentinel - Dev Mode${NC}"
echo -e "${CYAN}================================================${NC}"
echo ""

check_prereqs

# API Backend
start_service "api" \
    "$ROOT_DIR/services/api" \
    "cargo run" \
    "$GREEN"

# Attendre un peu que l'API demarre avant les bots
sleep 2

# Automod Bot
start_service "automod-bot" \
    "$ROOT_DIR/bots/automod-bot" \
    "cargo run" \
    "$BLUE"

# Ticket Bot
start_service "ticket-bot" \
    "$ROOT_DIR/bots/ticket-bot" \
    "cargo run" \
    "$BLUE"

# Desktop App
if [ -d "$ROOT_DIR/apps/desktop" ]; then
    if [ ! -d "$ROOT_DIR/apps/desktop/node_modules" ]; then
        echo -e "${YELLOW}[INSTALL] Desktop - npm install...${NC}"
        (cd "$ROOT_DIR/apps/desktop" && npm install) 2>&1
    fi
    start_service "desktop" \
        "$ROOT_DIR/apps/desktop" \
        "npm run tauri dev" \
        "$CYAN"
fi

echo ""
echo -e "${GREEN}================================================${NC}"
echo -e "${GREEN}   Tous les services sont lances !${NC}"
echo -e "${GREEN}================================================${NC}"
echo ""
echo -e "  API         : ${GREEN}http://localhost:3000${NC}"
echo -e "  Desktop     : ${CYAN}Tauri app (fenetre native)${NC}"
echo -e "  Logs        : ${YELLOW}.logs/*.log${NC}"
echo ""
echo -e "${YELLOW}Ctrl+C pour tout arreter${NC}"
echo ""

# Attendre que tous les processus tournent
wait
