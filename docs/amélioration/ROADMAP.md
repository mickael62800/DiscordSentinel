# 🗺️ Roadmap DiscordSentinel — Q2/Q3 2026

> **Date** : 2026-04-27
> **Horizon** : 12 semaines (3 mois) — 1 dev TPE
> **Mantra** : « **Sync first**. Toute feature ajoutée après la phase 1 est live-synced d'office. »
> **Objectif** : passer de ~55 % à ~95 % de couverture web admin et éliminer la divergence Discord ↔ Web.

---

## 🎯 Principes directeurs

1. **Foundation before features** — l'infra de sync (semaines 1-2) bénéficie à toutes les features suivantes.
2. **API-first, toujours** — pas de bypass, pas d'écriture DB depuis le web ou le bot direct.
3. **Pilote → généralisation** — tester chaque nouveau pattern sur 1 feature avant de l'étendre.
4. **No-regression** — chaque phase termine avec tests verts (api + bot + shared).
5. **Tier 1 d'abord** — les fondations admin (modération avancée + onboarding) avant le confort (slot leaderboard, etc.).

---

## 📅 Vue d'ensemble (12 semaines)

```
   Sem | Phase                              | Livrable principal
  ─────┼────────────────────────────────────┼────────────────────────────────
   1-2 | Phase 1 — Sync infra + pilote ban  | event bus + table mapping + SSE
   3   | Phase 2 — Welcome / Onboarding     | premiere impression nouveau membre
   4-5 | Phase 3 — Modération avancée       | evidence/review/strikes/notes
   6   | Phase 4 — Automod page             | UI config + logs détecteurs
   7   | Phase 5 — Voice channels complet   | thèmes, invites, whitelists, transfer
   8   | Phase 6 — Role panels CRUD         | création + edition depuis web
   9-10| Phase 7 — Coude avancé             | bounties/coalitions/vendettas/curses
   11  | Phase 8 — Levels & Conduct config  | sauvegarde + actions admin
   12  | Phase 9 — Polish & quick wins      | sponsorships, temp roles, slot/wheel UI
```

**Au-delà** : Q3 — gestion Discord native (channels, webhooks, server settings). Gros chantier à part.

### 🔄 Bonus livré : sync bilatérale **tickets**

En complément du pilote ban (qui n'avait pas de message Discord à éditer), un cas
d'usage **vraiment** bilatéral a été livré sur les **tickets** — c'est la
démonstration end-to-end du pattern :

- **Bot** (`panel.rs`) : enregistre le mapping `(ticket_uuid, "ticket", channel_id, welcome_msg_id)` après création.
- **API** : événement `ticket_closed` enrichi avec `action_id` (UUID) + `actor.source`.
- **Bot listener** (`handle_ticket_closed_from_web`) : si fermeture depuis web,
  - fetch le mapping via `GET /api/discord-messages/{action_id}`,
  - édite le welcome message (gris + footer 🔒 « Fermé via web »),
  - lock le channel (deny `SEND_MESSAGES` au `@everyone`),
  - rename `closed-<nom>`.
- **Web** (`useTickets`) : subscribe aux 4 events `ws:ticket_{created,closed,assigned,status_updated}` → refresh sans F5.
- **Idempotence** : si `actor.source != "web"`, le bot skip (pas de boucle).

---

## 🚀 Phase 1 — Sync infra + pilote ban (semaines 1-2)

> **Objectif** : tu cliques « Annuler » sur Discord → la web le voit en < 1s. Et inversement.

### 📦 Livrables

#### Semaine 1 — Infrastructure

- [x] **Migration SQL** `175_discord_action_messages.sql` ✅
  - Table `discord_action_messages` (action_id, kind, guild_id, channel_id, message_id, posted_at, last_edited_at)
  - PK composite `(action_id, kind)`, unique `(guild_id, channel_id, message_id)`
- [x] **Domain entity + ports** ✅
  - `DiscordActionMessage`, `NewDiscordActionMessage`, module `kinds`
  - Port outbound `DiscordActionMessageRepository`
  - Port inbound `ManageDiscordActionMessagesUseCase`
  - Service application `ManageDiscordActionMessagesService`
- [x] **Adapter Postgres** ✅ — `PgDiscordActionMessageRepository` (UPSERT idempotent)
- [x] **Endpoints HTTP** ✅
  - `POST /api/discord-messages/register`
  - `GET /api/discord-messages/{action_id}`
  - `DELETE /api/discord-messages/{action_id}/{kind}`
- [x] **AppState + bootstrap wiring** ✅
- ⚠️ ~~Event log + SSE~~ — **non nécessaire** : l'infra event bus existe déjà
  (Redis Stream `sentinel:events` + `EventBroadcaster` + `listen_stream_group`
  côté bot + WebSocket relay côté web via la gateway).
  → Phase 1 simplifiée : il suffit de **brancher** ce qui existe.

#### Semaine 2 — Pilote ban

- [x] **Bot — module `sync.rs`** ✅
  - `register_action_message()` (fire-and-forget vers `/api/discord-messages/register`)
  - `build_action_custom_id()` / `parse_action_custom_id()` (format `{namespace}:{verb}:{uuid}`)
  - Module `kinds` synchronisé avec le domain API
  - 4 tests unitaires (round-trip, namespace mismatch, bad UUID)
- [x] **API — émission events** ✅
  - `POST /api/moderation/execute-ban` accepte un `action_id` optionnel
    et publie `moderation.ban.executed { action_id, guild_id, target_id, actor }`
  - `DELETE /api/infractions/{id}` publie `moderation.ban.cancelled` si l'infraction est de type ban
- [x] **Web — subscribe events** ✅
  - `useBans.ts` consomme les events `ws:moderation.ban.{executed|cancelled|proposed}` via le bus local `@/api/events`
  - Filtre optimiste (retrait ligne sans refetch) + fallback refetch sur `proposed`
  - `executeBan(...)` passe l'`action_id` du proposal courant
- ⚠️ **Bot listener edit Discord** — **non applicable au cas ban** :
  les ban_proposals auto-créés par le worker `sync_ban_proposals` n'ont
  **pas de message Discord posté** (aucun handler bot ne les affiche).
  La table `discord_action_messages` reste vide pour le `kind=ban_proposal`
  tant qu'aucune commande Discord ne pose un embed. Le pattern listener
  sera testé sur la 1ʳᵉ feature qui a une vraie représentation Discord
  (tickets, roles panels, combats coude…).
- [ ] **Idempotence** : UPDATE conditionnel `WHERE status = 'pending'` —
  reporté en phase suivante (le `delete_infraction` actuel n'a pas de
  notion de statut, l'infraction est simplement supprimée).

### 🎯 Critères d'acceptation

- ✅ Exécuter un ban depuis le web → liste mise à jour en < 1s sans F5 (via WS)
- ✅ Annuler une proposition de ban depuis le web → ligne retirée live de la liste
- ✅ Pattern de sync utilisable pour la phase 2 (tickets) où les messages Discord existent vraiment
- ⏳ Edit message Discord à valider en phase suivante (cas où une commande Discord poste un embed)

### ⚠️ Risques

- **Bot rate-limit Discord** sur edits — throttler à 5 edits/sec
- **SSE saturé** sur grosse activité — filtrer côté serveur par guild + types
- **Compatibilité ascendante** — bans existants sans `action_id` mappé restent legacy (acceptable)

---

## 🏠 Phase 2 — Welcome / Onboarding (semaine 3) ✅

> **Objectif** : un admin configure le message de bienvenue, les rôles par défaut et le verification gate **depuis la web**, sans toucher Discord.

### 📦 Livrables

- [ ] **Migration SQL** `176_welcome_config.sql` (si pas déjà) — table `welcome_configs` (existe déjà côté repo `welcome_config_repo` à vérifier)
- [ ] **Endpoints API** :
  - `GET /api/welcome/{guild_id}` (existe — vérifier)
  - `PUT /api/welcome/{guild_id}` (existe — vérifier)
- [ ] **Page web** `apps/web/src/components/pages/WelcomePage.vue` :
  - Formulaire : message bienvenue (Markdown / embed builder), salon dédié, default roles (multi-select), verification gate (toggle + lien règles), bot mention
  - Aperçu live de l'embed (côté client, sans aller-retour)
  - Bouton "Test" qui poste le message dans un salon test
- [ ] **Bot** :
  - Sur `guild_member_addition` → lit la config welcome → poste le message
  - Si verification gate → assigne rôle "non vérifié" + attend la lecture des règles avant de promouvoir
- [ ] **Composable** `useWelcome.ts`
- [ ] **Service** `welcomeService.ts`
- [ ] **Sidebar** : ajouter le lien `/welcome` sous "Communauté"
- [ ] **Tests** : config save/load, message rendering, default roles assignment

### 🎯 Critères d'acceptation

- ✅ Admin configure un message + 2 rôles par défaut + verification gate depuis le web
- ✅ Un nouveau membre arrive → reçoit le message + rôles par défaut + statut "non vérifié"
- ✅ Membre lit/accepte les règles → rôle "vérifié" attribué automatiquement
- ✅ Test button poste un faux welcome dans un salon admin

---

## 🛡️ Phase 3 — Modération avancée (semaines 4-5) ✅
> Strikes, Notes, Reminders, Evidence, Reviews, Modstats — toutes les pages livrées.
> Templates (CRUD) reporté (rare usage admin, dispo via commande Discord `/template`).

> **Objectif** : le workflow modération complet (evidence, review, strikes, notes, reminders) est dans la web — fini les commandes Discord-only pour les modos.

### 📦 Livrables

#### Semaine 4 — Evidence + Review

- [ ] **Page** `/moderation/evidence` :
  - Pour chaque action listée, possibilité d'attacher des URLs/screenshots
  - Liste des preuves attachées par action
  - Endpoints : `POST/GET /api/moderation/evidence`
- [ ] **Page** `/moderation/review` :
  - Onglet "Mes demandes" : actions où j'ai demandé une review
  - Onglet "À traiter" (senior mods only) : reviews en attente
  - Statuts : Approved / Rejected / Changed avec notes
  - Endpoints : `POST /api/moderation/review`, `GET .../pending`, `PATCH .../resolve`
- [ ] **Sync** : events `review.requested`, `review.resolved` → notification staff

#### Semaine 5 — Strikes + Notes + Reminders

- [ ] **Page** `/moderation/strikes` :
  - Config par guild : seuils + actions auto (ex. 3 strikes → mute 1h, 5 strikes → ban)
  - Liste des strikes par user (visible aussi dans `/members > Surveillance`)
  - Reset manuel
  - Endpoints : `GET/PUT /api/strikes/config`, `GET .../{user_id}`, `DELETE .../{user_id}`
- [ ] **Section Notes** dans `/members > Surveillance` :
  - Édition/suppression des notes (lecture déjà OK)
  - Endpoints : `POST /api/notes`, `DELETE /api/notes/{id}`
- [ ] **Page** `/moderation/reminders` :
  - Création de rappels datés sur un user (ex. "vérifier ce user dans 30 jours")
  - Notification staff via salon dédié quand le rappel arrive
  - Endpoints : `POST/GET /api/reminders`
- [ ] **Page** `/moderation/templates` :
  - CRUD des templates de raisons partagés
  - Endpoints : déjà via commande Discord, ajouter en web

### 🎯 Critères d'acceptation

- ✅ Modo attache 3 screenshots à un ban → ils apparaissent live dans le panel review
- ✅ Senior mod résoud une review → l'embed Discord original est édité avec le verdict
- ✅ Strikes config sauvegardée → un user qui atteint 3 strikes est auto-muted

---

## 🤖 Phase 4 — Automod page (semaine 6) ✅

> **Objectif** : un admin configure et monitore l'automod **depuis la web**, sans CLI ou commande Discord obscure.

### 📦 Livrables

- [ ] **Page** `/security/automod` :
  - Onglet "Détecteurs" : toggle par détecteur (spam, links, phishing, mass-mention, regex custom)
  - Onglet "Règles" : seuils (msgs/sec, min compte âge, mots-clés, regex)
  - Onglet "Whitelists" : domaines, channels, roles exemptés
  - Onglet "Logs" : événements automod récents (qui, quoi, quand, action prise)
  - Onglet "Test" : dry-run d'un message contre les règles actuelles
- [ ] **Endpoints API** : exposer la config existante (deja en `bot_config` partial)
- [ ] **Sync** : events `automod.triggered`, `automod.action_taken`
- [ ] **Composable** `useAutomod.ts`

### 🎯 Critères d'acceptation

- ✅ Admin active "anti-phishing" depuis la web → bot le respecte sans redémarrage
- ✅ Logs affichent en live les détections (sans F5)
- ✅ Test dry-run : "salut connard" → le détecteur insulte fire en rouge

---

## 🎙️ Phase 5 — Voice channels complet (semaine 7) ✅ *(themes livrés ; whitelists/bans/invites/co-admins/transfer : services prêts pour la vue détail)*

> **Objectif** : la page `/voice-channels` couvre 100 % de l'API (thèmes, invites, whitelists, transfer, co-admins).

### 📦 Livrables

- [ ] **Sous-route** `/voice-channels/themes` :
  - CRUD des thèmes par guild (nom, icône, slow-mode default, bitrate, user limit)
  - Endpoints : `GET/POST/PATCH/DELETE /api/voice-channels/themes`
- [ ] **Sous-route** `/voice-channels/whitelists` :
  - Whitelists par owner (qui peut rejoindre quoi)
  - Bans par channel
  - Endpoints : `POST/DELETE /voice-channels/whitelist`, `POST/DELETE .../bans`
- [ ] **Sous-route** `/voice-channels/invites` :
  - Création d'invites custom avec link_id
  - Endpoints : `GET/POST/DELETE .../invites`
- [ ] **Action "Transfer ownership"** dans le détail channel :
  - Endpoint : `PATCH .../transfer`
- [ ] **Section "Co-admins"** dans le détail channel :
  - Ajouter / retirer co-admins
  - Endpoints : `POST/DELETE .../co-admins`
- [ ] **Sync** : events `voice.channel.created/updated/deleted/transferred`

### 🎯 Critères d'acceptation

- ✅ Admin crée un thème "Gaming Night" depuis la web → applicable instantanément
- ✅ Owner d'un channel transfère ownership depuis la web → l'autre user reçoit les permissions Discord
- ✅ Bans de channel synchronisés Discord ↔ Web

---

## 🎨 Phase 6 — Role panels CRUD (semaine 8)

> **Objectif** : créer/éditer/déployer les panels de rôles **depuis la web**, sans la commande Discord.

### 📦 Livrables

- [ ] **Page** `/role-panels` (refonte) :
  - Liste des panels existants avec état déploiement
  - Bouton "Nouveau panel" → modal :
    - Titre, description, mode (toggle/select-one/select-many), max_roles
    - Sélecteur de rôles via picker (multi)
    - Préview de l'embed
  - Bouton "Déployer" sur chaque panel → envoie sur Discord
- [ ] **Endpoints API** :
  - `POST /api/role-panels` (create)
  - `DELETE /api/role-panels/detail/{panel_id}`
  - `POST /api/auto-roles`, `DELETE .../{role_id}`
- [ ] **Sync** : events `panel.deployed`, `panel.role_added`, `panel.role_removed`

### 🎯 Critères d'acceptation

- ✅ Admin crée un panel "Pings" avec 5 rôles depuis la web → déploie sur Discord en 1 click
- ✅ Édition du panel → le message Discord est mis à jour automatiquement (via sync)
- ✅ Suppression du panel → le message Discord est supprimé

---

## 🎮 Phase 7 — Coude avancé (semaines 9-10)

> **Objectif** : exposer côté web tous les sous-systèmes Coude qui n'ont pas d'admin UI.

### 📦 Livrables (1 page consolidée à onglets : `/coude/avance`)

- [ ] **Onglet "Bounties"** : primes ouvertes, contributions, claims, historique régicides
- [ ] **Onglet "Coalitions"** : coalitions actives, membres, cibles
- [ ] **Onglet "Vendettas"** : vendettas en cours, gagnées, perdues
- [ ] **Onglet "Curses"** : malédictions actives par cible, levée admin
- [ ] **Onglet "Sabotages"** : sabotages actifs (graisser, empoisonner, fausse_assurance, pancarte)
- [ ] **Onglet "Insurance / Protections / Boosts"** : abonnements actifs par joueur
- [ ] **Onglet "Achievements"** : qui a quoi, top progression
- [ ] **Onglet "Prestige"** : liste des joueurs prestigés avec étoiles
- [ ] **Onglet "Saisons"** : config thème (bonus actifs), reset manuel, historique

Chaque onglet a son `useXxx` composable + appelle les endpoints API existants.

### 🎯 Critères d'acceptation

- ✅ Admin voit la liste des bounties ouvertes en live
- ✅ Admin peut lever une malédiction (admin override)
- ✅ Admin peut reset une saison ou changer son thème depuis la web

---

## 📈 Phase 8 — Levels & Conduct config (semaine 11)

> **Objectif** : sauvegarder la config et exposer les actions admin manquantes.

### 📦 Livrables

#### Levels

- [ ] **Page** `/levels` (refonte) :
  - Onglet "Config" : XP per message/voice, role multipliers, channel multipliers, decay, mode (separate/max/total)
  - Onglet "Actions" : add XP manuel, reset XP user/global
  - Onglet "Leaderboard" (existant)
  - Onglet "Rewards" (existant)
- [ ] **Endpoints** : `POST /api/levels/config`, `POST /api/levels/xp`

#### Conduct

- [ ] **Section "Config"** dans la page conduct existante :
  - Sauvegarde de la config (cooldown regen, seuils ban auto)
  - Endpoint : `POST /api/conduct/config`
- [ ] **Bouton "Sync ban proposals manuel"** : trigger `/sync-ban-proposals` à la demande

### 🎯 Critères d'acceptation

- ✅ Admin change `xp_per_voice_minute` → bot le respecte sans redémarrage
- ✅ Admin ajoute 500 XP à un user depuis le web → reflet immédiat dans `/levels`

---

## ✨ Phase 9 — Polish & quick wins (semaine 12)

> **Objectif** : finir les pages secondaires + intégration `Members` + dashboard ops.

### 📦 Livrables

- [ ] **Page** `/sponsorships` : liste parrains/filleuls, état, récompense XP
- [ ] **Page** `/temp-roles` : rôles temporaires actifs, ajout manuel, prolongation
- [ ] **Section "Name history"** dans `/members > Surveillance` : 10 derniers pseudos
- [ ] **Page** `/wheel` : recent spins toutes guilds, leaderboard, distribution cases
- [ ] **Page** `/slot` : recent spins, jackpot, RTP affiché
- [ ] **Section "Tables Blackjack"** : tables actives multijoueur
- [ ] **Page** `/system/operations` : models status, cache stats, AI jobs, exports jobs, bots health
- [ ] **Refacto SidebarNav** : groupes hiérarchisés (cf. WEB_ADMIN_GAPS.md § 7)
- [ ] **Quarantine + Appeals workflow** (si temps) : pages `/security/quarantine` + `/security/appeals`

### 🎯 Critères d'acceptation

- ✅ Sidebar passée de 22 liens plats à ~7 groupes hiérarchisés expansibles
- ✅ Toutes les nouvelles pages sont live-synced (validation en cliquant sur Discord, refresh web auto)

---

## 📊 Métriques de suivi

| KPI | Baseline (avant) | Cible (après 12 semaines) |
|---|---|---|
| **Pages web admin** | 22 | ~45 |
| **Endpoints API non consommés** | ~120 | < 30 (les bot-only) |
| **Couverture fonctionnelle** | ~55 % | ~90 % |
| **Délai action Discord → reflet web** | 1 F5 manuel | < 1s p95 |
| **Délai action web → reflet Discord** | 0 (pas de sync) | < 1s p95 |
| **% actions avec `action_id` mappé** | 0 % | > 99 % |
| **Tests api** | 2474 | 2700+ |
| **Tests bot** | 658 | 750+ |
| **% modération via web (vs Discord)** | ~30 % | > 70 % |

---

## ⚠️ Risques globaux & mitigations

| Risque | Impact | Mitigation |
|---|---|---|
| Phase 1 (sync) glisse | Toutes les phases bloquées | Time-box stricte 2 semaines. Si glisse, accepter version dégradée et avancer. |
| Bot Discord rate-limit | Edits perdus | Throttle global 5 req/s + queue retry. |
| SSE consomme trop de RAM | API instable | Limiter N connexions par user, filtrer aggressivement par guild. |
| Régressions sur features existantes | Confiance dégradée | Tests d'intégration + feature flags pour rollback rapide. |
| Scope creep (gestion Discord native) | Décalage 3+ mois | **Strictement hors roadmap Q2/Q3**. Réservé à Q4. |
| Modos refusent d'utiliser le web | ROI faible | Onboarding modo dédié + documentation après chaque phase. |

---

## 🚦 Décisions à prendre avant de démarrer

Cf. doc `SYNC_DISCORD_WEB_DESIGN.md` § 10 :

1. ✅ **Transport bot ↔ API events** : gRPC streaming
2. ✅ **Auth SSE web** : cookie session
3. ✅ **Rétention event_log** : 72h
4. ✅ **Format event** : JSON (SSE) + Protobuf (gRPC interne)
5. ✅ **Message Discord supprimé manuellement** : DELETE row mapping
6. ✅ **Backfill** : non, nouvelles actions seulement

Si un de ces points doit être discuté → faire la discussion avant la semaine 1.

---

## 🎯 Success criteria global

À la fin des 12 semaines, **un admin ouvre la web une fois par jour et n'a quasiment plus à toucher Discord pour modérer ou configurer**. Les commandes Discord deviennent un fallback / canal alternatif, pas le point d'entrée principal.

Métrique ultime : **le pourcentage de modérations exécutées via la web doit dépasser 70 %**.

---

## 📎 Références croisées

- `SYNC_DISCORD_WEB_DESIGN.md` — design technique de la sync (phase 1)
- `WEB_ADMIN_GAPS.md` — détail des manques (sources des phases 2-9)
- `COUDE_ARCHITECTURE_AUDIT.md` — état hexagonal API + dette logique métier bot

---

*Document de planification — à revoir après chaque phase pour ajuster.*
