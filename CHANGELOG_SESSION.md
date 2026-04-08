# DiscordSentinel — Changelog Session Claude

## Ce qui a ete fait

### 1. Logs et gestion d'erreurs (COMPLET)

**Objectif** : Plus aucune erreur silencieuse dans tout le projet.

- **API service** : 54 endroits corriges (`.ok()`, `let _ =`, `unwrap_or_default`) — tous avec `tracing::warn!` ou `tracing::error!`
- **automod-bot** : 11 corrections (handler.rs, discord_helpers.rs)
- **moderation-bot** : ~40 corrections (commands, handler, api_client)
- **security-bot** : 8 corrections
- **audit-bot** : 3 corrections
- **ticket-bot** : ~35 corrections (close.rs, interactions.rs, panel.rs, handler.rs)
- **community-bot** : ~12 corrections
- **progression-bot** : 4 corrections
- **coude-bot** : ~60 corrections (commands, handler, db.rs)
- **voice-bot** : ~33 corrections (handlers, interactions, setup)
- **roles-bot** : 8 corrections
- **image-bot** : 2 corrections
- **Workers** : coude-worker (10 corrections), gateway shutdown
- **Resultat final** : 0 `let _ = xxx.await`, 0 `.ok()` sans log, 0 `unwrap_or_default()` sur await sans log

### 2. Bugs et corrections critiques (COMPLET)

- **SQL injection securise** : `level_repository.rs` (debug_assert sur xp_col), `conduct_repository.rs` (log interval inconnu)
- **Infraction auto-ban vide** : `manage_conduct_service.rs` — champs remplis avec `system:conduct`
- **UUID nil silencieux** : `moderation.rs` — log si parse echoue
- **Race condition XP** : `manage_levels_service.rs` — error log si upsert echoue
- **N+1 member upsert** : `member_repository.rs` — batch dans une transaction
- **max_points hardcode** : `manage_members_service.rs` — lit la config guild
- **penalty_for_action** : `conduct.rs` — accepte `mute_temp`, `ban_permanent`, `ban_temp` + log action inconnue
- **Broadcaster silencieux** : log si serialisation JSON echoue
- **count_services Redis** : log chaque erreur Redis au lieu de `unwrap_or_default`
- **Redis KEYS → SCAN** : `redis_cache.rs` — `invalidate_pattern` utilise SCAN au lieu de KEYS (non-bloquant)
- **Events fantomes LevelsPage** : `level_up`/`xp_update` remplaces par `xp_gained` (le seul broadcast)

### 3. Securite API (COMPLET)

- **Validation inputs** : Module `validation.rs` centralise — Discord IDs (snowflake), longueurs max (reason 2000, content 4000, name 100), limit/offset >= 0
- **8 handlers proteges** : moderation, analyze, bot_config, tickets, security, notes, infractions, bot_persistence
- **Headers securite** : `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, `X-XSS-Protection`, `Referrer-Policy`, `Permissions-Policy`, `Cache-Control: no-store`
- **CORS restrictif** : Default securise (Tauri + localhost) au lieu de `AllowOrigin::any()`
- **API_KEY obligatoire en prod** : `REQUIRE_API_KEY=true` par defaut, refuse de demarrer si vide
- **Validation image** : Taille max 14 Mo + whitelist content_type
- **Ports DB/Redis** : Bindes sur `127.0.0.1` dans docker-compose (plus exposes au monde)

### 4. DRY / Refactoring (COMPLET)

#### bots/shared
- `config.rs` : Ajout `load_env()`, `load_env_optional()`, `load_env_bool()`, `load_env_string()`, `SimpleConfig`
- `api_client.rs` : Ajout helpers HTTP (`get_json`, `post_json`, `post_fire_and_forget`, `patch_fire_and_forget`, `delete_json`, `delete_with_body`)
- 5 bots simplifies avec `SimpleConfig` (moderation, progression, automod, community, roles)
- 5 bots simplifies avec `load_env`/`load_env_bool` (audit, security, image, ticket, voice)
- 10 api_client.rs migres vers les helpers HTTP (-24% de code, 513 lignes eliminees)

#### worker-common
- Ajout `load_database_url()`, `load_api_url()`, `load_redis_url()`, `load_env()`, `load_env_bool()`
- 6 workers simplifies
- cleanup-worker et coude-worker migres vers `spawn_periodic()`
- Gateway : `shutdown_signal()` importe depuis worker-common (plus de duplication)

### 5. WebSocket temps reel (COMPLET)

#### Nouvelles connexions WS
| Page | Events ecoutes |
|------|---------------|
| TicketsPage | `ticket_new/message/closed/assigned/status_updated/channel_updated` |
| SecurityPage | `security_event` |
| AuditPage | `log_entry_created` (nouveau broadcast ajoute) |
| ConductPage | `conduct_points_changed`, `user_zero_points` |
| WatchedUsersPage | `watched_user_added`, `infraction_new`, `security_event` |
| ModerationHubPage | + `strike_added`, `conduct_points_changed` |
| VoiceChannelsPage | + `voice_channel_updated`, `voice_invite_*` |
| Dashboard stats | `bot_heartbeat`, `stats_*`, `infraction_new`, `moderation_action` (debounce 2s) |
| Dashboard charts | `stats_*`, `infraction_new`, `moderation_action` (debounce 5s) |
| Analytics | `infraction_new`, `moderation_action` (debounce 10s) |
| Logs (toutes pages) | `log_entry_created` (debounce 2s) |

#### ConnectionBanner
- Polling HTTP `/health` 30s remplace par ecoute `bot_heartbeat` WS + fallback HTTP 90s

#### Notifications enrichies
- `log_entry_created` (error/warn) → toast rouge/orange + panneau notifications
- `user_zero_points`, `strike_added`, `watched_user_added` → panneau notifications

### 6. Toast notifications (COMPLET)

- **Systeme cree** : `useToast.ts` + `ToastContainer.vue` (success/error/warning/info, auto-dismiss, animation)
- **18 composables corriges** : toasts d'erreur user-visible + toasts de succes sur actions
- **7 pages corrigees** : confirmations dialog + toasts
- **Messages traduits** : 0 message anglais restant dans les composables/pages
- **Toasts automatiques** : erreurs bots/workers affichees via WS → toast

### 7. Nouveaux bots (EN COURS)

#### cleanup-bot (COMPLET)
- `/purge last|user|contains|bots|links|attachments` — Nettoyage messages Discord
- `/cleanup logs|infractions|audit` — Purge donnees BDD via API
- Endpoints API : `DELETE /api/purge/infractions`, `/audit-logs`, `/logs`
- Dockerfile + docker-compose

#### blackjack-bot (EN COURS)
- Structure bot creee (main.rs, handler.rs, config.rs, commands)
- Assets : 52 images de cartes en JPG
- **API creee** : endpoints start/hit/stand/double/active + logique de jeu complete dans `BlackjackService`
- **Reste** : Implementation des commandes Discord (api_client.rs, slash commands, embeds, boutons)

### 8. Wallet partage (EN COURS)

- **Table `user_wallets`** : creee avec migration SQL (+ migration depuis coude_players)
- **Table `wallet_transactions`** : historique de toutes les operations
- **API endpoints** : GET wallet, POST credit/debit, POST transfer, GET leaderboard, GET transactions
- **BlackjackService** : utilise le wallet pour les mises/gains
- **Reste** : Migration du coude-bot pour utiliser le wallet au lieu de coude_players.coins

### 9. Docker-compose (MIS A JOUR)

- `API_KEY` obligatoire (plus de `:-` fallback vide)
- `REQUIRE_API_KEY=true` ajoute
- `REDIS_URL` + `REDIS_CHANNEL` ajoutes pour moderation-bot et ticket-bot
- Ports Postgres/Redis bindes sur `127.0.0.1`
- cleanup-bot et blackjack-bot ajoutes
- `.env` : `REQUIRE_API_KEY=false` pour le dev local
- `.env.example` : documente
- `.gitignore` : `!.env.example` accepte

---

## Ce qui reste a faire

### Priorite 1 — Terminer le blackjack-bot

- [ ] `api_client.rs` : Methodes pour appeler les endpoints blackjack + wallet
- [ ] `commands/blackjack.rs` : Slash commands `/blackjack play|hit|stand|double|balance`
- [ ] Boutons interactifs (Hit/Stand/Double) via ComponentInteraction
- [ ] Affichage des cartes en embed (texte Unicode ♠♥♦♣ + scores)
- [ ] Gestion des erreurs + toasts cote desktop si broadcast

### Priorite 2 — Migrer coude-bot vers l'API

C'est le plus gros chantier restant (~5000 lignes a refactorer).

#### Pourquoi cette migration est necessaire

Le coude-bot est le **seul bot** qui accede directement a PostgreSQL (`db.rs` avec `sqlx`).
Tous les autres bots passent par l'API REST. Ce couplage direct pose plusieurs problemes :

1. **Contourne la validation** : Les validations d'inputs ajoutees dans l'API (snowflake IDs, longueurs max, etc.) sont ignorees par le coude-bot qui ecrit directement en base.
2. **Pas de logs API** : Le middleware `api_logger` ne voit pas les operations du coude-bot — impossible de tracer les actions dans le dashboard de logs.
3. **Pas de broadcast WebSocket** : Les events temps reel (`wallet_credit`, `wallet_debit`, etc.) ne sont pas emis quand le coude-bot modifie la base directement. L'app desktop ne recoit aucune mise a jour en temps reel pour le jeu coude.
4. **Pas de rate limiting** : Aucune protection contre l'abus — un utilisateur peut spammer des commandes sans limite.
5. **Coins isoles** : Les coins du coude sont dans `coude_players.coins`, pas dans le wallet partage `user_wallets`. Le blackjack et les futurs jeux ne peuvent pas partager le meme portefeuille.
6. **Couplage schema DB** : Si le schema change (ajout de colonne, renommage), il faut modifier le bot ET l'API. Avec l'API comme seul point d'acces, seule l'API doit etre mise a jour.
7. **Securite** : Le bot a un acces direct en lecture/ecriture a TOUTE la base de donnees. Si le bot est compromis, c'est toute la BDD qui est exposee. Via l'API, il n'a acces qu'aux endpoints autorises.

#### Taches

- [ ] Creer ~40 endpoints API pour TOUTES les operations de `db.rs` (combats, paris, shop, vol, casino, primes, inventory, cooldowns, XP, events, leaderboards)
- [ ] Creer `api_client.rs` dans le coude-bot (remplace `db.rs`)
- [ ] Mettre a jour les 15 fichiers de commandes pour utiliser `api_client` au lieu de `db`
- [ ] Mettre a jour `handler.rs` et `main.rs` (supprimer PgPool, utiliser ApiClient)
- [ ] Migrer les coins vers le wallet partage (credit/debit au lieu de `UPDATE coude_players SET coins`)
- [ ] Supprimer `db.rs` et la dependance `sqlx` du bot
- [ ] Adapter le `coude-worker` pour utiliser l'API aussi (ou le garder en acces DB direct vu que c'est un worker backend)

### Priorite 3 — Ameliorations futures

- [ ] **RBAC** : Controle d'acces par role (au lieu d'un seul API_KEY pour tout)
- [ ] **Guild-level auth** : Verifier que le caller a le droit sur le guild_id demande
- [ ] **Indexes DB** : Ajouter les index manquants identifies dans l'audit (guild_id, action_type LIKE, etc.)
- [ ] **Health check cache** : Mettre en cache `/health` pendant 5-10s pour reduire la charge
- [ ] **Compression gzip** : Ajouter `tower-http` compression sur les reponses API
- [ ] **Auth WebSocket** : Verifier le token sur les connexions WS du gateway
- [ ] **Pagination security events** : Ajouter limit/offset au lieu du LIMIT 200 hardcode
- [ ] **Idempotency keys** : Pour les endpoints fire-and-forget (eviter les doublons sur retry)
- [ ] **Audit logging** : Logger QUI a fait QUELLE action (pas juste "desktop")
- [ ] **Migration coude-worker** : Meme refactor que le bot (DB direct → API), moins urgent car c'est un worker backend
