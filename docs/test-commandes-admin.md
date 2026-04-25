# Tests — Commandes admin Discord (bot + API)

Checklist de tests manuels sur Discord pour valider les commandes réservées aux modérateurs et admins. Les commandes passent toujours par le bot `sentinel-bot` qui appelle l'API (`services/api`). Un résultat OK implique que **les deux** aient fonctionné.

> Avant de tester : vérifier que le bot est en ligne, que l'API `/health` répond 200, et que ton compte Discord a le rôle/permission attendu sur le serveur de test.

**Légende** :
- ✅ = comportement attendu (cas nominal)
- ⚠️ = cas limite / edge case
- 🔒 = vérification RBAC (doit être rejeté)
- 📡 = vérif côté API (log, broadcast, DB)

---

## 1. Modération — actions directes

### `/ban <user> <reason> [duration]`
Permission : `BAN_MEMBERS`. `duration` en heures (vide = permanent).

- [ ] ✅ `/ban @testuser "spam continu"` → user banni, embed confirmation éphémère, action en DB `moderation_actions`
- [ ] ✅ `/ban @testuser "test temp" 1` → ban de 1h, rappel auto créé dans `sanction_reminders` (T-24h ? vérifier si dur > 24h)
- [ ] 🔒 User sans `BAN_MEMBERS` → réponse "Permission BAN_MEMBERS requise"
- [ ] ⚠️ `/ban @bot_proprio` → protection propriétaire (si implémentée) ou erreur Discord
- [ ] 📡 Broadcast `moderation_action` reçu côté gateway WS
- [ ] 📡 Si strike franchit un seuil → broadcast `strike_added` avec `escalation_action`

### `/unban <user_id>`
Permission : `BAN_MEMBERS`. `user_id` = Snowflake Discord (pas mention).

- [ ] ✅ `/unban 1234567890` sur un user banni → unban OK
- [ ] ⚠️ ID invalide (lettres) → erreur validation
- [ ] ⚠️ ID d'un user non banni → erreur Discord (404 ban not found)
- [ ] 🔒 Sans `BAN_MEMBERS` → refus

### `/mute <user> <reason> [duration]`
Permission : `MODERATE_MEMBERS`. `duration` en minutes (vide = 60 par défaut, max 40320 = 28 jours).

- [ ] ✅ `/mute @testuser "flood"` → timeout 1h (défaut via `resolve_mute_duration`)
- [ ] ✅ `/mute @testuser "test" 5` → timeout 5 min
- [ ] ⚠️ `/mute @testuser "x" 0` → 0 minute = pas de timeout (valeur explicite préservée)
- [ ] ⚠️ `/mute @testuser "x" 50000` → clamp Discord à 28j ou erreur
- [ ] 🔒 Sans `MODERATE_MEMBERS` → refus
- [ ] 📡 Reminder auto pour mute temporaire

### `/unmute <user>`
- [ ] ✅ `/unmute @muted_user` → retire le timeout Discord
- [ ] ⚠️ User non muté → message "pas de timeout actif"

### `/warn <user> <gravity> <reason>`
`gravity` ∈ `low | medium | high`.

- [ ] ✅ `/warn @testuser low "petite remarque"` → DM au user + log DB
- [ ] ✅ `/warn @testuser high "insulte grave"` → strike ajouté, vérifier escalade si seuil atteint
- [ ] ⚠️ 3 `/warn high` d'affilée → escalade auto (mute ou ban selon config)
- [ ] 📡 `GET /api/strikes/<guild>/<user>` retourne `active_count` incrémenté

### `/unwarn <action_id>`
- [ ] ✅ Annule un warn avec son UUID d'action
- [ ] ⚠️ UUID invalide → erreur 400

### `/massmute <users> <reason> [duration]`
`users` = IDs séparés par espaces ou virgules.

- [ ] ✅ `/massmute "111 222 333" "raid" 60` → 3 mutes
- [ ] ⚠️ Mélange `/massmute "111, 222, 333"` avec virgules → doit parser aussi
- [ ] ⚠️ `/massmute "111 abc 333" "x"` → skip `abc`, traite les 2 autres, rapporte les erreurs
- [ ] 🔒 Sans `MODERATE_MEMBERS` → refus

### `/massban <users> <reason>`
- [ ] ✅ `/massban "111 222" "alt accounts"` → 2 bans
- [ ] 🔒 Sans `BAN_MEMBERS` → refus

---

## 2. Modération — consultation

### `/history <user>`
- [ ] ✅ `/history @testuser` → embed avec warns/mutes/bans des X derniers mois
- [ ] ⚠️ User sans historique → message "aucune action"

### `/modstats` (stats modérateurs)
- [ ] ✅ Embed top 20 modérateurs par nombre d'actions (30 derniers jours)
- [ ] ⚠️ Guild sans actions → liste vide

### `/expirations`
- [ ] ✅ Liste les sanctions qui expirent dans les prochaines 24h

### `/note add|list|del <user>`
- [ ] ✅ `/note add @user "anniversaire le 12/03"` → note ajoutée
- [ ] ✅ `/note list @user` → affiche toutes les notes
- [ ] ✅ `/note del <note_id>` → suppression

### `/review add|list|resolve`
- [ ] ✅ `/review add <action_id>` → ajoute une action à la queue de review
- [ ] ✅ `/review list` → liste les pending
- [ ] ✅ `/review resolve <review_id> approved "bonne decision"` → marque comme résolu

### `/evidence add|list <action_id>`
- [ ] ✅ `/evidence add <action_id> <url>` → attache une preuve
- [ ] ⚠️ URL vide ou > 2000 chars → erreur validation (`validate_evidence_url`)
- [ ] ✅ `/evidence list <action_id>` → liste les preuves

### `/template list|show|apply`
- [ ] ✅ Templates de raisons pré-configurés : `/template list`, `/template apply <name> @user`

### `/call <user> <reason>`
- [ ] ✅ Convoque un user par DM avec raison
- [ ] ⚠️ DM désactivés côté user → fallback mention en channel

### `/compare <user1> <user2>`
- [ ] ✅ Compare les historiques de 2 users (alt detection)

### `/context <user>`
- [ ] ✅ Contexte récent (messages, channels actifs, rôles) pour un user

### `/transcript <channel>`
- [ ] ✅ Export des messages d'un salon (ticket ou général)

### `/export <type>`
- [ ] ✅ Export asynchrone (délégué à `export-worker`) → retourne un `job_id`

### `/appeal <action_id>`
- [ ] ✅ Un user sanctionné peut faire appel → crée une entrée en review

---

## 3. Sécurité — `/security`

Permission : admin (configurable via RBAC).

- [ ] ✅ `/security lockdown on|off` → active/désactive le mode lockdown (nouveaux membres mutés auto)
- [ ] ✅ `/security quarantaine @user` → met un user en quarantaine (rôle `@Quarantaine`)
- [ ] ✅ `/security alts @user` → détecte les alt accounts (même IP, même device fingerprint)
- [ ] ✅ `/security raidmode on|off` → captcha forcé pour tous les nouveaux arrivants
- [ ] 📡 Events `security_event` broadcastés sur Redis
- [ ] 🔒 User non admin → refus RBAC

---

## 4. Audit — `/audit`

Permission : modérateur+.

- [ ] ✅ `/audit list [limit]` → derniers événements audit Discord (via `audit_logs` partitionnée)
- [ ] ✅ `/audit user @user` → actions ayant ciblé ou émis par ce user
- [ ] ✅ `/audit anomaly` → détections de patterns anormaux (watched_users, weekly_report)
- [ ] ✅ `/audit report` → génère le rapport hebdo
- [ ] 📡 Vérifier que `discord-audit-sync-worker` ingère bien les events en continu

---

## 5. Cleanup — `/cleanup` et `/purge`

### `/purge <count> [user]`
Permission : `MANAGE_MESSAGES`.

- [ ] ✅ `/purge 10` → supprime les 10 derniers messages du salon
- [ ] ✅ `/purge 50 @user` → supprime les 50 derniers messages de ce user (bulk delete)
- [ ] ⚠️ `/purge 0` ou `/purge 200` → clampage ou erreur (max Discord = 100 via bulk)
- [ ] ⚠️ Messages > 14 jours → impossible via bulk (API Discord), suppression un par un ou erreur
- [ ] 🔒 Sans `MANAGE_MESSAGES` → refus

### `/cleanup <channel> [days]`
Commande admin plus large (rétention ciblée).

- [ ] ✅ `/cleanup #salon 30` → purge les messages de ce salon > 30 jours
- [ ] 🔒 Admin uniquement

---

## 6. Automod — `/automod`

Permission : admin.

- [ ] ✅ `/automod status` → état actuel (seuils IA texte/vision, dampening, context_format)
- [ ] ✅ `/automod text on|off` → toggle détection IA texte
- [ ] ✅ `/automod vision on|off` → toggle détection IA vision
- [ ] ✅ `/automod threshold text 0.75` → règle le seuil texte
- [ ] ✅ `/automod threshold vision 0.80` → règle le seuil vision
- [ ] ⚠️ Seuil hors [0, 1] → erreur validation
- [ ] 📡 Changements reflétés immédiatement dans `bot_guild_config` (bot_name=`automod-bot`)
- [ ] 📡 L'API `POST /analyze` tient compte des nouveaux seuils

---

## 7. Tickets — `/ticket-admin`

Permission : staff tickets.

- [ ] ✅ `/ticket-admin list open` → liste les tickets ouverts
- [ ] ✅ `/ticket-admin assign <ticket_id> @staff` → assigne à un membre du staff
- [ ] ✅ `/ticket-admin close <ticket_id> "resolved"` → clôture avec motif
- [ ] ✅ `/ticket-admin transcript <ticket_id>` → exporte le transcript
- [ ] ✅ `/ticket-admin sla` → dashboard SLA (temps de réponse moyen, dépassements)

---

## 8. Games — `/game-admin`

- [ ] ✅ `/game-admin reset <game_type>` → reset des scores/stats d'un jeu
- [ ] ✅ `/game-admin config <param> <value>` → config rapide
- [ ] 🔒 User non admin → refus

### `/blackjack-setup`
- [ ] ✅ Crée une table blackjack persistante dans un salon
- [ ] 🔒 Admin uniquement

---

## 9. Coup de Coude — admin

### `/reset-stats @user`
- [ ] ✅ Reset les stats d'un joueur coude (admin)
- [ ] 🔒 User non admin → refus

### Via API / Desktop : `DELETE /api/coude/<guild>/purge`
- [ ] ✅ Depuis le web panel, purge TOTAL du sous-système coude (7 tables via `COUDE_PURGE_TABLES`)
- [ ] ⚠️ Double confirmation frontend obligatoire
- [ ] 🔒 RBAC moderator+ (check_role_for_guild)
- [ ] 📡 Retour JSON `{"coude_bets": 12, "coude_combats": 5, ...}`

### `/saison admin`
- [ ] ✅ Clôture ou démarre une saison

### `/taunts-channel #salon`
- [ ] ✅ Définit le salon où les taunts sont postés
- [ ] 🔒 Admin uniquement

---

## 10. Tests transverses — API directe (curl)

Pour valider sans passer par Discord (endpoints HTTP directs).

**Auth** : `Authorization: Bearer $API_KEY`

### Santé & métriques
- [ ] `curl http://localhost:3000/health` → `200 OK`
- [ ] `curl http://localhost:3000/metrics` → expose les counters Prometheus
- [ ] `curl http://localhost:3001/health` (gateway) → `200 OK`

### Wallet (admin reset)
- [ ] `POST /api/wallet/<guild>/<user>/reset` body `{"new_balance": 500}` → wallet reset à 500
- [ ] `POST /api/wallet/<guild>/reset-all` body `{}` → bulk reset à 100 (défaut)
- [ ] Broadcast `wallet_reset` / `wallet_reset_all` reçu sur gateway WS

### Moderation
- [ ] `POST /api/moderation/execute-ban` body `{guild_id, user_id, reason}` → ban Discord + log
- [ ] `POST /api/moderation/execute-mute` body `{guild_id, user_id, reason, duration}` → timeout
- [ ] `POST /api/moderation/execute-mute` sans `duration` → 1h par défaut (`resolve_mute_duration`)
- [ ] `DELETE /api/moderation/actions/<uuid>` sur un ban → unban auto + suppression DB

### Coude purge
- [ ] `DELETE /api/coude/<guild>/purge` → 7 tables vidées, retour des counts

### RBAC multi-tenant
- [ ] Requête sans `X-Discord-Token` → pass-through (bot/internal)
- [ ] Requête avec `X-Discord-Token` d'un user non membre de `<guild>` → **403**
- [ ] Requête avec token d'un user membre mais rôle insuffisant → **403** avec message rôle requis

---

## 11. Workers (tâches périodiques à observer)

Ces workers n'ont pas de commandes Discord mais leurs effets sont visibles.

- [ ] `moderation-worker` : 24h avant expiration d'un ban/mute → DM rappel au modérateur
- [ ] `temp-roles-worker` : rôle temporaire expiré → retrait auto + event Redis
- [ ] `coude-worker` : combat en phase `betting` expiré → résolution auto + payout paris
- [ ] `analytics-worker` : snapshot quotidien présent dans `user_stats` chaque matin
- [ ] `cache-worker` : MV `mv_*_leaderboard` refreshée toutes les 5 min
- [ ] `cleanup-worker` : rétention DB appliquée (vérifier vieilles lignes `logs` purgées)

---

## 12. Scénarios end-to-end

### E2E-1 : Ban temporaire complet
1. `/ban @testuser "spam" 2` (2h)
2. Vérifier ban Discord actif
3. Vérifier `moderation_actions` en DB + `sanction_reminders` créé
4. Attendre 2h (ou triche via `UPDATE expires_at`)
5. `moderation-worker` doit déclencher l'unban auto → user débanni
6. Broadcast `sanction_expiry_reminder` reçu

### E2E-2 : Escalade par warns
1. `/warn @user high "x"` × 3 (ou selon seuil configuré)
2. Au 3e warn : escalade auto (mute/ban)
3. Broadcast `strike_added` avec `escalation_action` non null
4. Vérifier que `StrikeResult::should_trigger_escalation_broadcast()` a bien émis

### E2E-3 : Purge coude depuis web
1. Depuis le dashboard web : page admin coude → bouton "Purge complète"
2. Double confirmation modal
3. API appelée avec `X-Discord-Token`
4. RBAC vérifie moderator+ sur la guild
5. 7 tables vidées atomiquement
6. JSON de retour affiché dans l'UI

### E2E-4 : Ticket complet
1. User : `/ticket` → crée un ticket
2. Staff : `/ticket-admin assign` à soi-même
3. Échange de messages
4. Staff : `/ticket-admin close <id> "resolved"`
5. Transcript généré automatiquement
6. User reçoit un sondage satisfaction

---

## 13. Checklist avant prod

- [ ] Toutes les sections 1–9 passent sur un serveur de staging avec un compte `testuser`
- [ ] `cargo test --lib` (services/api) → 100% pass
- [ ] `docker compose up -d` → tous les services `healthy`
- [ ] Prometheus montre tous les `up{}` à 1
- [ ] Aucun panic/error dans `docker compose logs` sur 10 min de trafic test
- [ ] Grafana dashboard "API overview" → latence p99 < 500ms sur les endpoints moderation/wallet
