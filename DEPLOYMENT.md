# DiscordSentinel - Guide de deploiement et securisation

## Prerequis serveur

| Ressource | Minimum | Recommande |
|-----------|---------|------------|
| RAM | 8 Go | 16 Go |
| CPU | 2 coeurs | 4 coeurs (Xeon) |
| Reseau | 50 Mbps | 100 Mbps symetrique |
| OS | Ubuntu 22.04+ / Debian 12+ | Ubuntu 24.04 LTS |
| Docker | 24+ | Derniere version stable |
| Docker Compose | v2+ | Derniere version stable |

---

## 1. Installation initiale

### 1.1 Cloner le projet

```bash
git clone https://github.com/mickael62800/DiscordSentinel.git
cd DiscordSentinel
```

### 1.2 Configurer le .env

```bash
cp .env.example .env
nano .env
```

Variables obligatoires :

```env
# PostgreSQL
POSTGRES_PASSWORD=un_mot_de_passe_fort

# Redis
REDIS_PASSWORD=un_autre_mot_de_passe_fort

# API
API_KEY=une_cle_api_longue_et_aleatoire

# Tokens Discord (un par bot)
AUTOMOD_DISCORD_TOKEN=...
MODERATION_DISCORD_TOKEN=...
SECURITY_DISCORD_TOKEN=...
TICKET_DISCORD_TOKEN=...
IMAGE_DISCORD_TOKEN=...
VOICE_DISCORD_TOKEN=...
PROGRESSION_DISCORD_TOKEN=...
AUDIT_DISCORD_TOKEN=...
COMMUNITY_DISCORD_TOKEN=...
ROLES_DISCORD_TOKEN=...
COUDE_DISCORD_TOKEN=...

# Voice bot
VOICE_GUILD_ID=...
VOICE_PUBLIC_CREATOR_CHANNEL_ID=...
VOICE_PRIVATE_CREATOR_CHANNEL_ID=...
```

### 1.3 Build et lancement

```bash
# Build sequentiel (evite la saturation RAM)
bash build-all.sh

# Lancement dans l'ordre (infra -> API -> workers -> bots)
bash start-all.sh

# Verification
bash health-check.sh
```

### 1.4 Seed des donnees

```bash
# Regles de moderation par defaut
bash seed-rules.sh
```

---

## 2. Securisation pour acces Internet

### 2.1 Firewall (ufw)

```bash
# Activer le firewall
sudo ufw default deny incoming
sudo ufw default allow outgoing

# SSH (changer le port si possible)
sudo ufw allow 22/tcp

# HTTPS uniquement (reverse proxy)
sudo ufw allow 443/tcp

# Optionnel : HTTP pour redirection vers HTTPS
sudo ufw allow 80/tcp

# Activer
sudo ufw enable
sudo ufw status
```

**IMPORTANT** : ne jamais ouvrir les ports 3000, 3001, 5432, 6379 au monde.

### 2.2 Reverse proxy avec Caddy (HTTPS automatique)

Caddy gere automatiquement les certificats Let's Encrypt.

#### Installation

```bash
sudo apt install -y debian-keyring debian-archive-keyring apt-transport-https curl
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list
sudo apt update
sudo apt install caddy
```

#### Configuration

Creer `/etc/caddy/Caddyfile` :

```caddyfile
# Remplacer par votre domaine
sentinel.votre-domaine.com {
    # API Backend
    handle /api/* {
        reverse_proxy localhost:3000
    }

    handle /health {
        reverse_proxy localhost:3000
    }

    handle /rules {
        reverse_proxy localhost:3000
    }

    # WebSocket Gateway
    handle /ws {
        reverse_proxy localhost:3001
    }

    # Rate limiting global (necessite module)
    # rate_limit {remote.ip} 100r/m
}
```

```bash
sudo systemctl enable caddy
sudo systemctl restart caddy
```

L'HTTPS est automatique. Caddy obtient et renouvelle les certificats tout seul.

### 2.3 Forcer l'API_KEY

Dans le `.env`, definir une cle forte :

```bash
# Generer une cle aleatoire
openssl rand -hex 32
```

```env
API_KEY=votre_cle_generee_ici
```

L'app bureau envoie cette cle en header `Authorization: Bearer <API_KEY>`.
Toute requete sans ce header est rejetee par l'API.

### 2.4 Restreindre Docker aux ports internes

Dans `docker-compose.yml`, ne pas exposer les ports sur `0.0.0.0`.
Remplacer :

```yaml
ports:
  - "3000:3000"    # Accessible depuis l'exterieur
```

Par :

```yaml
ports:
  - "127.0.0.1:3000:3000"    # Uniquement accessible en local
```

Appliquer a tous les services :

| Service | Port | Binding |
|---------|------|---------|
| API | 3000 | `127.0.0.1:3000:3000` |
| Gateway | 3001 | `127.0.0.1:3001:3001` |
| PostgreSQL | 5432 | `127.0.0.1:5432:5432` |
| Redis | 6379 | `127.0.0.1:6379:6379` |

### 2.5 Securiser SSH

```bash
# Editer la config SSH
sudo nano /etc/ssh/sshd_config
```

Modifications recommandees :

```
# Desactiver l'auth par mot de passe (cle SSH uniquement)
PasswordAuthentication no
PubkeyAuthentication yes

# Desactiver root login
PermitRootLogin no

# Changer le port (optionnel mais recommande)
Port 2222

# Limiter les tentatives
MaxAuthTries 3
```

```bash
sudo systemctl restart sshd
```

### 2.6 Fail2ban (protection brute-force)

```bash
sudo apt install fail2ban
sudo systemctl enable fail2ban
```

Creer `/etc/fail2ban/jail.local` :

```ini
[sshd]
enabled = true
port = 2222
maxretry = 3
bantime = 3600
findtime = 600

[caddy-auth]
enabled = true
port = 443
filter = caddy-auth
logpath = /var/log/caddy/access.log
maxretry = 10
bantime = 600
```

---

## 3. Configuration PostgreSQL optimisee

Deja configure dans `docker-compose.yml` :

```yaml
command: postgres -c max_connections=150 -c shared_buffers=256MB -c effective_cache_size=4GB
```

### Sauvegardes automatiques

Ajouter un cron pour les backups :

```bash
crontab -e
```

```cron
# Backup quotidien a 3h du matin
0 3 * * * docker exec sentinel-postgres pg_dump -U sentinel discord_sentinel | gzip > /backups/sentinel_$(date +\%Y\%m\%d).sql.gz

# Retention 30 jours
0 4 * * * find /backups -name "sentinel_*.sql.gz" -mtime +30 -delete
```

```bash
sudo mkdir -p /backups
```

---

## 4. Monitoring

### 4.1 Health check automatique

```bash
crontab -e
```

```cron
# Verification toutes les 5 minutes
*/5 * * * * cd /chemin/vers/DiscordSentinel && bash health-check.sh >> /var/log/sentinel-health.log 2>&1
```

### 4.2 Alertes Discord (optionnel)

Creer un webhook Discord pour recevoir les alertes :

```bash
# Exemple de notification en cas de probleme
curl -H "Content-Type: application/json" \
  -d '{"content":"⚠️ DiscordSentinel: un service est down!"}' \
  https://discord.com/api/webhooks/VOTRE_WEBHOOK_ID/VOTRE_WEBHOOK_TOKEN
```

### 4.3 Metriques a surveiller

| Metrique | Seuil d'alerte | Commande |
|----------|---------------|----------|
| RAM | > 12 Go (75%) | `free -m` |
| CPU | > 80% soutenu | `top -bn1` |
| Connexions DB | > 120/150 | `SELECT count(*) FROM pg_stat_activity;` |
| Espace disque | > 80% | `df -h` |
| Conteneurs down | > 0 | `docker compose ps` |

---

## 5. Mises a jour

### 5.1 Mettre a jour l'application

```bash
cd /chemin/vers/DiscordSentinel
git pull

# Rebuild uniquement ce qui a change
bash build-all.sh

# Relancer
bash start-all.sh
```

### 5.2 Mettre a jour le systeme

```bash
sudo apt update && sudo apt upgrade -y

# Mettre a jour Docker
sudo apt install --only-upgrade docker-ce docker-ce-cli
```

---

## 6. Checklist de deploiement

- [ ] `.env` configure avec des mots de passe forts
- [ ] `bash build-all.sh` sans erreur
- [ ] `bash start-all.sh` — tous les services UP
- [ ] `bash seed-rules.sh` — regles de moderation creees
- [ ] `bash health-check.sh` — tout OK
- [ ] Firewall (ufw) actif — seuls ports 22/443 ouverts
- [ ] Caddy installe avec HTTPS et reverse proxy
- [ ] Ports Docker en `127.0.0.1` (pas exposes au monde)
- [ ] SSH securise (cle uniquement, pas de root)
- [ ] Fail2ban actif
- [ ] Backup PostgreSQL quotidien configure
- [ ] Health check en cron toutes les 5 minutes
- [ ] App bureau configuree avec `https://sentinel.votre-domaine.com`

---

## 7. Architecture reseau

```
Internet
   |
   | (port 443 HTTPS)
   v
[Caddy - Reverse Proxy]
   |
   |--- /api/*    --> localhost:3000 (API)
   |--- /health   --> localhost:3000 (API)
   |--- /rules    --> localhost:3000 (API)
   |--- /ws       --> localhost:3001 (Gateway WebSocket)
   |
   v
[Docker Network interne]
   |
   |--- sentinel-api (3000)
   |--- sentinel-gateway (3001)
   |--- sentinel-postgres (5432) -- jamais expose
   |--- sentinel-redis (6379) -- jamais expose
   |--- sentinel-*-bot (pas de port)
   |--- sentinel-*-worker (pas de port)
```

L'app bureau se connecte uniquement a `https://sentinel.votre-domaine.com`.
Tous les autres services communiquent via le reseau Docker interne.
