#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────────
# Script de tests DiscordSentinel
# Lance PostgreSQL + Redis via Docker, execute les
# migrations, puis les tests unitaires + integration.
# ─────────────────────────────────────────────────

COMPOSE_FILE="docker-compose.test.yml"
DB_URL="postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test"
REDIS_URL="redis://localhost:6380"

# Couleurs
GREEN='\033[0;32m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { echo -e "${CYAN}[INFO]${NC} $*"; }
ok()    { echo -e "${GREEN}[OK]${NC} $*"; }
fail()  { echo -e "${RED}[FAIL]${NC} $*"; }

cleanup() {
    info "Arret des conteneurs de test..."
    docker compose -f "$COMPOSE_FILE" down -v --remove-orphans 2>/dev/null || true
}
trap cleanup EXIT

# ── 1. Demarrer les services ──
info "Demarrage PostgreSQL + Redis de test..."
docker compose -f "$COMPOSE_FILE" up -d --wait

ok "Services demarres (postgres:5433, redis:6380)"

# ── 2. Lancer les migrations ──
info "Application des migrations..."
export DATABASE_URL="$DB_URL"
cd services/api
cargo sqlx migrate run --source ./migrations 2>/dev/null || {
    # Si sqlx-cli n'est pas installe, utiliser psql directement
    info "sqlx-cli non disponible, application manuelle des migrations..."
    for f in migrations/*.sql; do
        psql "$DB_URL" -f "$f" 2>/dev/null || true
    done
}
cd ../..
ok "Migrations appliquees"

# ── 3. Tests unitaires (tous les bots + API) ──
info "Tests unitaires..."
FAILED=0

for bot in automod-bot security-bot moderation-bot audit-bot voice-bot ticket-bot community-bot progression-bot coude-bot blackjack-bot; do
    if [ -f "bots/$bot/Cargo.toml" ]; then
        info "  $bot..."
        if cargo test --manifest-path "bots/$bot/Cargo.toml" --quiet 2>&1; then
            ok "  $bot"
        else
            fail "  $bot"
            FAILED=$((FAILED + 1))
        fi
    fi
done

info "  API (lib)..."
if cargo test --manifest-path services/api/Cargo.toml --lib --quiet 2>&1; then
    ok "  API (lib)"
else
    fail "  API (lib)"
    FAILED=$((FAILED + 1))
fi

# ── 4. Tests d'integration HTTP (API) ──
info "Tests d'integration HTTP..."
export DATABASE_URL="$DB_URL"
export REDIS_URL="$REDIS_URL"
export API_KEY=""
export REQUIRE_API_KEY="false"

if cargo test --manifest-path services/api/Cargo.toml --tests --quiet 2>&1; then
    ok "Tests d'integration HTTP"
else
    fail "Tests d'integration HTTP"
    FAILED=$((FAILED + 1))
fi

# ── 5. Resultat ──
echo ""
if [ "$FAILED" -eq 0 ]; then
    ok "Tous les tests passent !"
    exit 0
else
    fail "$FAILED suite(s) de tests en echec"
    exit 1
fi
