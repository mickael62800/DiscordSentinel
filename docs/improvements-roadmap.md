# Roadmap d'ameliorations — DiscordSentinel

Date : 02/04/2026

---

## Workers a creer

### 1. Cache Worker ✅ FAIT

**Objectif :** Pre-calculer les donnees lourdes et les stocker en Redis pour que les endpoints API repondent instantanement.

**Ce qu'il fait :**
- Toutes les 5 minutes : pre-calcule les analytics (heatmap, tendances, top infracteurs, pics d'activite) et les stocke en Redis
- Toutes les 10 minutes : pre-calcule les statistiques du dashboard (stats globales, top membres, activite vocale)
- Toutes les heures : agrege les voice_sessions en stats par salon (top salons, duree moyenne, pics)
- Au demarrage : chauffe le cache avec les donnees les plus demandees (guild list, bot definitions)

**Impact :**
- Les pages Dashboard et Analytics chargent en < 50ms au lieu de 500ms-2s
- Reduit la charge PostgreSQL de ~80% sur les requetes d'agregation
- Les 5 requetes paralleles du endpoint `/api/analytics` deviennent un simple GET Redis

**Configuration :**
- `analytics_refresh_secs` (defaut: 300)
- `dashboard_refresh_secs` (defaut: 600)
- `voice_stats_refresh_secs` (defaut: 3600)
- `cache_warming_enabled` (defaut: true)

**Priorite : MEDIUM** — ✅ IMPLEMENTE (sentinel-cache-worker, 3 jobs, 6 tests)

---

### 2. Cleanup Worker ✅ FAIT

**Objectif :** Nettoyer automatiquement les donnees anciennes pour eviter que la base grossisse indefiniment.

**Ce qu'il fait :**
- Toutes les heures : supprime les voice_sessions de plus de X jours (configurable, defaut 90)
- Toutes les heures : supprime les logs API de plus de X jours (configurable, defaut 30)
- Tous les jours : supprime les ticket_messages des tickets fermes depuis plus de X jours (configurable, defaut 180)
- Tous les jours : supprime les entrees de _sqlx_migrations orphelines
- Tous les jours : nettoie les cles Redis expirees (SCAN + verification)
- Toutes les semaines : VACUUM ANALYZE sur les tables les plus volumineuses

**Impact :**
- Empeche la base de grossir sans limite
- Maintient les performances des requetes SQL (index fragmentation)
- Libere l'espace disque automatiquement

**Configuration :**
- `voice_sessions_retention_days` (defaut: 90)
- `logs_retention_days` (defaut: 30)
- `closed_tickets_retention_days` (defaut: 180)
- `vacuum_enabled` (defaut: true)
- `cleanup_interval_hours` (defaut: 1)

**Priorite : LOW** — ✅ IMPLEMENTE (sentinel-cleanup-worker, 2 jobs, 4 tests)

---

## Optimisations code (sans nouveau worker)

### Performance

| # | Amelioration | Impact | Effort | Priorite | Statut |
|---|-------------|--------|--------|----------|--------|
| 1 | ~~Cache Redis sur `/api/analytics` (heatmap, trends)~~ | ~~Les 5 requetes paralleles prennent 500ms+~~ | ~~Moyen~~ | ~~MEDIUM~~ | ✅ FAIT — TTL 5min, cache-first sur les 6 endpoints |
| 2 | ~~Pagination cote API (pas seulement cote client)~~ | ~~Evite de charger 10 000+ infractions en memoire~~ | ~~Moyen~~ | ~~MEDIUM~~ | ✅ FAIT — tickets, bans, watched_users, voice_channels (limit/offset SQL) |
| 3 | ~~Batch API pour watched users (audit-bot fait N appels/guild)~~ | ~~Reduit 10+ appels API toutes les 60s a 1 seul~~ | ~~Faible~~ | ~~MEDIUM~~ | ✅ FAIT — 1 appel batch sans filtre guild + watched_set.clear() |
| 4 | ~~Index PostgreSQL sur `infractions(guild_id, created_at)`~~ | ~~Accelere les requetes filtrees par date~~ | ~~Faible~~ | ~~LOW~~ | ✅ FAIT — migration 058 : 4 index composites (infractions, mod_actions, audit_logs) |
| 5 | ~~Connection pool Redis (actuellement multiplexed mais pas pool)~~ | ~~Meilleur throughput sous charge~~ | ~~Faible~~ | ~~LOW~~ | ✅ FAIT — connexion persistante dans RedisCache, plus de get_connection() par requete |

### Qualite de code

| # | Amelioration | Impact | Effort | Priorite | Statut |
|---|-------------|--------|--------|----------|--------|
| 6 | ~~Splitter `ticket.rs` (1686 lignes) en sous-modules~~ | ~~Maintenabilite, lisibilite~~ | ~~Eleve~~ | ~~MEDIUM~~ | ✅ FAIT — 6 modules : constants, panel, close, interactions, helpers, mod.rs |
| 7 | ~~Splitter `WatchedUsersPage.vue` (1184 lignes)~~ | ~~Extraire modales en composants~~ | ~~Moyen~~ | ~~LOW~~ | ✅ FAIT — 3 composants extraits : ActivityTimeline, AddWatchModal, UserDossierPanel |
| 8 | ~~Extraire les modales en composants reutilisables (ban, surveillance)~~ | ~~Reutilisabilite, DRY~~ | ~~Moyen~~ | ~~LOW~~ | ✅ FAIT — BanModal.vue extrait, utilise par BansPage + ModerationHubPage |
| 9 | ~~Ajouter des tests unitaires pour stats-bot (7 actuellement)~~ | ~~Couverture de tests~~ | ~~Faible~~ | ~~LOW~~ | ✅ 26 tests |
| 10 | ~~Ajouter des tests pour les 4 workers~~ | ~~Couverture de tests~~ | ~~Moyen~~ | ~~LOW~~ | ✅ 18 tests |
| 11 | Tests E2E Discord (commande /ticket test integree) | Detection de regressions sur les interactions | Eleve | MEDIUM | |

### Architecture

| # | Amelioration | Impact | Effort | Priorite | Statut |
|---|-------------|--------|--------|----------|--------|
| 12 | ~~Enums compile-time pour statuts/priorites (au lieu de strings)~~ | ~~Validation type-safe, pas de fautes de frappe~~ | ~~Moyen~~ | ~~MEDIUM~~ | ✅ FAIT — TicketStatus, TicketPriority, ModerationActionType + 16 tests |
| 13 | ~~Monitoring cache hit/miss Redis~~ | ~~Savoir si le cache est efficace~~ | ~~Faible~~ | ~~LOW~~ | ✅ FAIT — compteurs atomiques hit/miss dans RedisCache + endpoint GET /api/cache/stats |
| 14 | ~~Rate limiting granulaire par endpoint (pas global)~~ | ~~Proteger les endpoints lourds~~ | ~~Moyen~~ | ~~LOW~~ | ✅ FAIT — heavy_limiter (5 req/s) sur /analyze, /analyze/image, /api/analytics |
| 15 | Backup serveur Discord (roles, salons, permissions) | Restauration en cas de crash | Eleve | MEDIUM | |

### Desktop

| # | Amelioration | Impact | Effort | Priorite | Statut |
|---|-------------|--------|--------|----------|--------|
| 16 | Page Membres (refonte ConductPage → MembersPage) | Vue unifiee des membres avec date entree/sortie, stats, filtres | Eleve | HIGH | |
| 19 | Export CSV/PDF des donnees (infractions, tickets, analytics) | Reporting pour les admins | Moyen | LOW | |

### Ameliorations recentes (hors roadmap initiale)

| Composant | Amelioration | Statut |
|-----------|-------------|--------|
| API ML (Python) | Logging structure, validation Pydantic Field(), CORS securise, enum ModelType, Dockerfile, refactor training en sous-fonctions | ✅ FAIT |
| API ML (Python) | 84 tests pytest (endpoints, validation, metrics, utils, constantes) | ✅ FAIT |
| stats-bot | Fix panics options[0]→.first(), check enabled, logging erreurs XP, validation clamp, progress_bar safe | ✅ FAIT |
| image-bot | Fix panics unwrap/expect, logging 5x erreurs Discord, constantes extraites, helper embed, safe WEBP detection | ✅ FAIT |
| Workers (x4) | Fix expect→process::exit, erreurs loguees (let_→warn!), constantes nommees, debug→warn snapshots | ✅ FAIT |

---

## Etat actuel du projet

| Metrique | Valeur |
|----------|--------|
| Lignes Rust | ~46 500 |
| Lignes Vue/TS | ~15 700 |
| Lignes Python | ~600 |
| Tests | **692 pass** |
| Erreurs | 0 |
| Warnings | 0 |
| Bots | 10 |
| Workers | **6** |
| Endpoints API | ~61 |
| Params config | 260+ |
| Note globale | **8.5/10** |
