#!/bin/bash
# ============================================================================
# Setup helpers HOST pour la page Securite serveur de DiscordSentinel.
#
# A lancer en root sur l'host (PAS dans un conteneur).
#
# Usage :
#   sudo bash infra/scripts/setup-host-security.sh fail2ban
#   sudo bash infra/scripts/setup-host-security.sh all
#
# Modules disponibles :
#   fail2ban  : installe fail2ban + jail SSH/nginx + cron export JSON
#   all       : tous les modules ci-dessus
# ============================================================================

set -euo pipefail

SENTINEL_DATA_DIR="/var/lib/sentinel"
SCRIPT_DIR="/usr/local/bin"

require_root() {
    if [ "$EUID" -ne 0 ]; then
        echo "❌ Ce script doit être lancé en root (sudo)."
        exit 1
    fi
}

ensure_data_dir() {
    mkdir -p "$SENTINEL_DATA_DIR"
    chmod 755 "$SENTINEL_DATA_DIR"
}

# ── Module fail2ban ─────────────────────────────────────────────────────

setup_fail2ban() {
    echo "🛡  Setup fail2ban"
    echo ""

    # 1. Installation
    if ! command -v fail2ban-client &>/dev/null; then
        echo "[1/4] Installation fail2ban…"
        apt-get update -qq
        apt-get install -y fail2ban
    else
        echo "[1/4] fail2ban déjà installé ✓"
    fi

    # 2. Configuration jail SSH (si pas déjà configurée)
    if [ ! -f /etc/fail2ban/jail.local ]; then
        echo "[2/4] Création jail.local (SSH protection)…"
        cat > /etc/fail2ban/jail.local <<'JAIL_EOF'
[DEFAULT]
# Bantime : 1h, ré-attempts max 5 sur 10min
bantime  = 1h
findtime = 10m
maxretry = 5
# Whitelist LAN privé
ignoreip = 127.0.0.1/8 ::1 192.168.0.0/16 10.0.0.0/8 172.16.0.0/12

[sshd]
enabled = true
port    = ssh
backend = systemd
maxretry = 3
bantime  = 1h

# Decommente si tu veux bannir aussi sur attaques nginx
# [nginx-http-auth]
# enabled = true
# port    = http,https
# logpath = /var/log/nginx/error.log
# maxretry = 5
JAIL_EOF
        systemctl restart fail2ban
    else
        echo "[2/4] jail.local déjà présent (pas écrasé) ✓"
    fi

    systemctl enable --now fail2ban

    # 3. Script export JSON pour l'API DiscordSentinel
    echo "[3/4] Création script export $SCRIPT_DIR/fail2ban-export.sh…"
    cat > "$SCRIPT_DIR/fail2ban-export.sh" <<'EXPORT_EOF'
#!/bin/bash
# Export fail2ban status -> JSON (lu par l'API DiscordSentinel)
# Genere par setup-host-security.sh, ne pas modifier manuellement.
set -eu
OUT=/var/lib/sentinel/fail2ban-status.json
mkdir -p /var/lib/sentinel
chmod 755 /var/lib/sentinel

JAILS=$(fail2ban-client status 2>/dev/null | grep "Jail list:" | sed 's/.*://;s/,/ /g')

{
    echo "{"
    echo "  \"updated_at\": \"$(date -Iseconds)\","
    echo "  \"jails\": ["
    FIRST=1
    for JAIL in $JAILS; do
        JAIL=$(echo "$JAIL" | xargs)
        [ -z "$JAIL" ] && continue
        BANNED=$(fail2ban-client status "$JAIL" 2>/dev/null | grep "Banned IP list:" | sed 's/.*://' | xargs || true)
        TOTAL=$(fail2ban-client status "$JAIL" 2>/dev/null | grep "Total banned:" | sed 's/.*://' | xargs || true)
        [ $FIRST -eq 0 ] && echo "    ,"
        echo "    {"
        echo "      \"name\": \"$JAIL\","
        echo "      \"total_banned\": ${TOTAL:-0},"
        echo "      \"banned_ips\": \"${BANNED:-}\""
        echo "    }"
        FIRST=0
    done
    echo "  ]"
    echo "}"
} > "$OUT.tmp" && mv "$OUT.tmp" "$OUT"

chmod 644 "$OUT"
EXPORT_EOF
    chmod +x "$SCRIPT_DIR/fail2ban-export.sh"

    # Premier export
    "$SCRIPT_DIR/fail2ban-export.sh"

    # 4. Cron toutes les 2 min
    echo "[4/4] Cron /etc/cron.d/fail2ban-export (toutes les 2 min)…"
    echo "*/2 * * * * root /usr/local/bin/fail2ban-export.sh" > /etc/cron.d/fail2ban-export
    chmod 644 /etc/cron.d/fail2ban-export

    echo ""
    echo "✅ fail2ban configuré."
    echo ""
    echo "Vérifications :"
    echo "  systemctl status fail2ban --no-pager"
    echo "  fail2ban-client status"
    echo "  cat /var/lib/sentinel/fail2ban-status.json"
    echo ""
    echo "Pour que l'API DiscordSentinel le voie :"
    echo "  cd /home/\$USER/DiscordSentinel/infra/docker"
    echo "  docker compose up -d --force-recreate api"
    echo ""
}

# ── Dispatcher ──────────────────────────────────────────────────────────

main() {
    require_root
    ensure_data_dir

    case "${1:-help}" in
        fail2ban)
            setup_fail2ban
            ;;
        all)
            setup_fail2ban
            ;;
        help|--help|-h)
            cat <<HELP
Usage: sudo bash setup-host-security.sh <module>

Modules disponibles :
  fail2ban  Installation fail2ban + jail SSH + cron export JSON pour l'API
  all       Tous les modules

Exemples :
  sudo bash setup-host-security.sh fail2ban
  sudo bash setup-host-security.sh all
HELP
            ;;
        *)
            echo "❌ Module inconnu : $1"
            echo "Lance 'sudo bash setup-host-security.sh help' pour la liste."
            exit 1
            ;;
    esac
}

main "$@"
