# Haute disponibilite et Failover — DiscordSentinel

## Problematique

Si le serveur principal plante et ne redemarrage pas, toute la plateforme est hors service : API, bots Discord, base de donnees. Ce document decrit les strategies pour assurer la continuite de service.

---

## Architecture cible

```
                     DNS / Cloudflare
                          |
                     Load Balancer
                    /              \
            Serveur A              Serveur B
            (principal)            (standby)
            ┌─────────┐           ┌─────────┐
            │ API      │           │ API      │ (eteint)
            │ 6 bots   │           │ 6 bots   │ (eteint)
            │ Postgres │──replica──│ Postgres │
            │ Redis    │──replica──│ Redis    │
            └─────────┘           └─────────┘
                 │                      │
                 └── Health Checker ────┘
                     (ping toutes les 10s)
```

**Principe** : Serveur A fait tourner tout. Serveur B est pret a prendre le relais. Un health checker surveille A. Si A ne repond plus → B se lance automatiquement.

---

## Niveaux de protection

### Niveau 1 — Redemarrage automatique (actuel)

**Cout** : 0€ | **Coupure** : 5-30s | **Protection** : crash process

Docker Compose avec `restart: unless-stopped` sur tous les services. Si un bot ou l'API crash, Docker le relance automatiquement.

**Deja en place** dans `docker-compose.yml`.

Limites : ne protege pas contre un crash serveur complet (hardware, OS, reseau).

---

### Niveau 2 — Monitoring + Alertes

**Cout** : 0€ | **Coupure** : N/A (detection) | **Protection** : visibilite

Surveiller l'etat de chaque service et alerter en cas de probleme.

**Composants** :
- Endpoint `/health` sur l'API (deja en place) — verifie PostgreSQL, Redis, API
- Heartbeats des bots (deja en place) — chaque bot ping l'API toutes les 30s
- Dashboard desktop affiche l'etat en temps reel (deja en place)

**A ajouter** :
- Webhook Discord pour alerter un salon quand un service est down
- Endpoint `/ready` qui verifie API + DB + Redis + au moins 1 bot connecte

---

### Niveau 3 — Failover actif-passif (2 serveurs)

**Cout** : 5-15€/mois (2eme VPS) | **Coupure** : 10-30s | **Protection** : crash serveur complet

#### Infrastructure requise

| Composant | Serveur A (principal) | Serveur B (standby) |
|-----------|----------------------|---------------------|
| API | Active | Pret (eteint) |
| 6 bots | Actifs | Prets (eteints) |
| PostgreSQL | Primaire | Replica streaming |
| Redis | Primaire | Replica |
| Docker | Compose up | Compose installe |

#### PostgreSQL Streaming Replication

Serveur A (`postgresql.conf`) :
```
wal_level = replica
max_wal_senders = 3
```

Serveur A (`pg_hba.conf`) :
```
host replication replicator serveur_b_ip/32 md5
```

Serveur B :
```bash
pg_basebackup -h serveur_a_ip -D /var/lib/postgresql/data -U replicator -P -R
```

La replication est en temps reel. Serveur B a une copie exacte de la DB.

#### Redis Replication

Serveur B (`redis.conf`) :
```
replicaof serveur_a_ip 6379
masterauth sentinel_redis
```

#### Script de failover (sur Serveur B)

```bash
#!/bin/bash
# failover-watcher.sh — A executer en permanence sur Serveur B

API_URL="http://serveur_a_ip:3000/health"
CHECK_INTERVAL=10      # Secondes entre chaque check
FAIL_THRESHOLD=3       # Nombre d'echecs avant failover
FAIL_COUNT=0

echo "Failover watcher demarre — surveillance de $API_URL"

while true; do
    if curl -sf --max-time 5 "$API_URL" > /dev/null 2>&1; then
        FAIL_COUNT=0
    else
        FAIL_COUNT=$((FAIL_COUNT + 1))
        echo "$(date) — Health check echoue ($FAIL_COUNT/$FAIL_THRESHOLD)"

        if [ $FAIL_COUNT -ge $FAIL_THRESHOLD ]; then
            echo "$(date) — FAILOVER ACTIVE — Demarrage des services sur Serveur B"

            # 1. Promouvoir PostgreSQL replica en primaire
            pg_ctlcluster 16 main promote

            # 2. Arreter la replication Redis
            redis-cli -a sentinel_redis REPLICAOF NO ONE

            # 3. Lancer tous les services
            cd /opt/DiscordSentinel
            docker compose up -d

            echo "$(date) — Serveur B est maintenant le serveur principal"

            # 4. Notifier via Discord webhook
            curl -X POST "$DISCORD_WEBHOOK_URL" \
                 -H "Content-Type: application/json" \
                 -d '{"content": "⚠️ **FAILOVER ACTIVE** — Serveur A down, Serveur B a pris le relais."}'

            # Arreter la boucle — on est maintenant le principal
            break
        fi
    fi

    sleep $CHECK_INTERVAL
done
```

#### Particularite des bots Discord

Un token Discord ne peut avoir qu'**une seule connexion gateway**. Les bots sur B doivent rester **eteints** tant que A fonctionne. Au failover, B les lance et Discord reconnecte en ~5s.

Si les bots sur A sont toujours connectes (serveur A partiellement up), il y aura un conflit. Solution : le script de failover tente d'abord d'eteindre les bots sur A via SSH avant de lancer ceux sur B.

```bash
# Tentative d'arret propre sur A (optionnel, timeout 5s)
ssh -o ConnectTimeout=5 serveur_a "cd /opt/DiscordSentinel && docker compose down" 2>/dev/null || true
```

---

### Niveau 4 — Docker Swarm (multi-noeud automatique)

**Cout** : 10-20€/mois (2+ VPS) | **Coupure** : 5-10s | **Protection** : crash serveur + auto-healing

Docker Swarm gere automatiquement le failover des containers entre les noeuds.

#### Setup

Sur Serveur A (manager) :
```bash
docker swarm init --advertise-addr serveur_a_ip
```

Sur Serveur B (worker) :
```bash
docker swarm join --token <token> serveur_a_ip:2377
```

#### docker-compose.swarm.yml

```yaml
version: "3.8"
services:
  api:
    image: sentinel-api:latest
    deploy:
      replicas: 2
      restart_policy:
        condition: any
      placement:
        max_replicas_per_node: 1
    # ...

  automod-bot:
    image: sentinel-automod-bot:latest
    deploy:
      replicas: 1          # Un seul par token
      restart_policy:
        condition: any
    # ...
```

Deployer :
```bash
docker stack deploy -c docker-compose.swarm.yml sentinel
```

Si un noeud tombe, Swarm relance automatiquement les services sur l'autre noeud.

**Avantages** : automatique, pas de script custom
**Limites** : PostgreSQL et Redis doivent etre externalises (pas dans Swarm) pour eviter la perte de donnees

---

### Niveau 5 — Kubernetes (enterprise)

**Cout** : 50€+/mois (cluster manage) | **Coupure** : quasi-zero | **Protection** : totale

Pour un deploiement production a grande echelle. Non detaille ici car premature pour le projet actuel.

Services recommandes : DigitalOcean Kubernetes, OVH Managed K8s, Google GKE.

---

## Recommandation par etape

| Etape | Niveau | Action | Priorite |
|-------|--------|--------|----------|
| 1 | 1 | Deployer via Docker Compose (deja fait) | ✅ Fait |
| 2 | 2 | Ajouter webhook Discord pour alertes down | Haute |
| 3 | 2 | Ajouter endpoint `/ready` complet | Haute |
| 4 | 3 | Louer 2eme VPS + setup replication PostgreSQL | Moyenne |
| 5 | 3 | Deployer script failover-watcher sur Serveur B | Moyenne |
| 6 | 4 | Migrer vers Docker Swarm si 3+ serveurs | Basse |

---

## Hebergeurs recommandes

| Hebergeur | Prix/mois | Specs | Localisation |
|-----------|-----------|-------|-------------|
| Hetzner | 4-8€ | 2 vCPU, 4GB RAM | Allemagne, Finlande |
| OVH | 5-12€ | 2 vCPU, 4GB RAM | France |
| Contabo | 6-10€ | 4 vCPU, 8GB RAM | Allemagne |
| DigitalOcean | 12-24$ | 2 vCPU, 4GB RAM | Amsterdam, Frankfurt |

Pour DiscordSentinel, un VPS avec 2 vCPU et 4GB RAM est suffisant pour faire tourner l'API + les 6 bots + PostgreSQL + Redis.

---

## Checklist avant deploiement production

- [ ] Docker Compose teste et fonctionnel
- [ ] `.env` securise (pas dans git, permissions 600)
- [ ] HTTPS avec certificat (Let's Encrypt via Traefik ou nginx)
- [ ] Backup PostgreSQL automatique (pg_dump quotidien)
- [ ] Firewall configure (seuls ports 80/443 ouverts)
- [ ] Monitoring en place (health checks + alertes)
- [ ] Procedure de failover documentee et testee
- [ ] Tokens Discord securises (variables d'environnement, pas en dur)
