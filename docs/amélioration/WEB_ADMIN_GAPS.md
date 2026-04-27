# 🌐 Web admin — Audit des manques pour gérer un serveur Discord de bout en bout

> **Date** : 2026-04-27
> **Méthode** : 2 agents d'analyse parallèles (inventaire pages web + diff endpoints API ↔ services web)
> **Objectif** : identifier ce qui manque dans la web admin pour qu'un owner/admin puisse gérer son serveur Discord intégralement sans repasser par Discord ou la DB.

---

## 1. 📊 État actuel

### 22 pages admin existantes (couverture par module)

| Module | Pages | Couvert |
|---|---|---|
| 📊 Dashboard | `/`, `/logs` | ✅ vue globale + logs |
| 🛡️ Modération | `/moderation`, `/members`, `/rules` | ✅ partielle (manque review/evidence) |
| 🔒 Sécurité | `/security`, `/audit` | ✅ lecture |
| 🎫 Communauté | `/tickets`, `/voice-channels`, `/role-panels`, `/levels` | ⚠️ partiel |
| 🎮 Jeux | `/games`, `/coude`, `/blackjack`, `/wallet`, `/tournaments`, `/taunts` | ✅ admin OK |
| ⚙️ Config | `/discord-roles`, `/component-config`, `/rbac`, `/settings` | ✅ |

### Volumétrie diff API ↔ Web
- **~120 endpoints API** non consommés par le web (inventaire détaillé en annexe).
- **22 pages** existantes, dont la majorité sont fonctionnelles.
- **~30 pages potentielles** identifiées comme manquantes pour une gestion complète.

---

## 2. 🚨 Manques critiques (P0) — bloquent une gestion complète

### 🛡️ MOD-1. Workflow de modération avancée (evidence + review)

L'API a tout — le web n'a rien.

**Ce qui manque** :
- **Page `/moderation/evidence`** : attacher / lister des preuves (URLs, screenshots) sur une action.
- **Page `/moderation/review`** : workflow de validation à deux niveaux. Modos demandent une review, seniors valident/rejettent.
- **Templates** : éditer les templates de raisons partagés.
- **Modstats dashboard** : qui modère le plus, ratios, cooldowns.

**Endpoints prêts** : `POST /api/moderation/evidence`, `GET .../review/pending`, `PATCH .../review/{id}/resolve`.

### 🤖 MOD-2. Page Automod & rules custom

**Aucune page Automod** dans la web admin alors que le module existe et a un `config_schema` riche (anti-spam, anti-link, anti-phishing, anti-mention…).

**Ce qui manque** :
- Toggle des détecteurs (spam, links, phishing, mass-mention…).
- Seuils par détecteur (messages/sec, regex personnalisés, whitelists).
- Test dry-run en direct (déjà dispo via `/automod test message` côté Discord).
- Logs d'actions automod (qu'est-ce qui a été détecté, et quoi en a été fait).

**À créer** : page `/automod` + `useAutomod` composable.

### 🌟 MOD-3. Strikes, notes, reminders

3 systèmes API déjà en place, **0 page web**.

**Ce qui manque** :
- **`/strikes`** : config par guild (seuils + actions auto), liste des strikes par user, reset manuel.
- **`/notes`** : notes internes par membre (déjà visibles dans `/members > Surveillance` mais pas éditables).
- **`/reminders`** : rappels modo programmés (« ce user revient sur le serveur dans 30 jours »).

**Endpoints prêts** : tout `/api/strikes/*`, `/api/notes/*`, `/api/reminders/*`.

### 👋 SRV-1. Welcome / Onboarding

Endpoints `GET/PUT /api/welcome/{guild_id}` existent — **aucune page**.

**Ce qui manque** :
- Configuration du message de bienvenue.
- Default roles à l'arrivée (déjà dans le bot mais pas configurable web).
- Verification gate (anti-bot, captcha, lecture de règles).
- Aperçu live de l'embed de welcome.

**Critique** : c'est la première impression d'un nouveau membre — **incontournable**.

### 🎙️ COM-1. Voice channels — gestion complète

La page existe mais ne couvre que la lecture + cleanup. **Énorme gap** sur l'admin.

**Endpoints prêts non utilisés** :
- Whitelists par owner (`POST/DELETE /voice-channels/whitelist`)
- Bans par channel (`POST/DELETE .../bans/{user_id}`)
- Invites custom (`GET/POST/DELETE .../invites`)
- Themes (`GET/POST/PATCH/DELETE .../themes`)
- Transfer ownership (`PATCH .../transfer`)
- Co-admins (`POST/DELETE .../co-admins`)

**Ce qui manque** : section « Gestion avancée » sur la page voice-channels existante, OU sous-routes `/voice-channels/themes`, `/voice-channels/whitelists`.

### 🎨 COM-2. Role panels — CRUD complet

Aujourd'hui : **lecture seule** côté web. La création se fait par script ou DB direct.

**Endpoints prêts non utilisés** :
- `POST /api/role-panels` — créer un panel
- `DELETE /api/role-panels/detail/{panel_id}`
- `POST /api/auto-roles`, `DELETE /api/auto-roles/{guild_id}/{role_id}`

**Ce qui manque** : modal/page de création (titre, mode, max_roles, sélection de rôles via picker), édition, déploiement directement depuis le web (au lieu de la commande Discord `/roles-panel deploy`).

---

## 3. ⚠️ Manques importants (P1)

### 🎭 GAME-1. Coude — features sans UI admin

Le module Coude a 50+ migrations et plein de sous-systèmes. Le web couvre `/coude`, `/wallet`, `/tournaments`, `/taunts` — mais des pans entiers manquent :

| Sous-système | Endpoints API | Manque côté web |
|---|---|---|
| **Bounties / primes collectives** | OK | Liste des primes ouvertes, contributions, claims |
| **Coalitions** | OK | Liste des coalitions actives, membres, cibles |
| **Vendettas** | OK | Vendettas en cours, gagnées, perdues |
| **Curses / malédictions** | OK | Liste des malédictions actives, levée admin |
| **Sabotages** | OK | Tableau des sabotages actifs |
| **Insurance / steal protections / boosts** | OK | Abonnements actifs par joueur |
| **Memorial des clodos** | OK | Page publique-friendly |
| **Roue du destin** | OK | Recent spins, leaderboard |
| **Slot machine** | OK | Recent spins, jackpot, leaderboard |
| **Saisons coude** | OK | Configuration thème, reset manuel |
| **Achievements** | OK | Vue admin pour vérifier qui a quoi |
| **Prestige** | OK | Liste des prestiges, étoiles |

**Recommandation** : créer une section `/coude/avance` avec sous-onglets pour chaque sous-système.

### 📈 LVL-1. Levels — config + actions

Page `/levels` existe pour le leaderboard et les rewards, mais :

**Manque** :
- Sauvegarde de la config (`POST /api/levels/config` non consommé) — XP per message, per voice minute, role multipliers, channel multipliers, decay…
- Ajout XP manuel par admin (`POST /api/levels/xp`) — outil de modération XP.
- Reset XP individuel ou global.
- Liste des `xp_role_multipliers` et `xp_channel_multipliers` éditables.

### 💰 ECO-1. Wallet — transfer admin + transactions

Page `/wallet` permet credit/debit/reset mais :

**Manque** :
- **Transfer admin** entre 2 joueurs (`POST /api/wallet/transfer` non consommé) — utile pour corriger une erreur.
- **Historique transactions** (`GET .../transactions`) — voir d'où viennent les coins.

### 🎫 TICK-1. Tickets — création + cycle complet

Page `/tickets` permet la lecture + reply + close. Manque :

- **Créer un ticket** depuis le web (utile pour ouvrir un ticket de la part d'un user qu'on a contacté en DM).
- **Changer le statut** (open → in_progress → waiting_user → closed).
- **Ré-assigner** à un autre staff (déjà partiel).

### 🔐 SEC-1. Quarantine + appeal workflow

L'API gère déjà du quarantine et le commande `/appeal` existe côté Discord, mais :

**Manque** :
- Page `/security/quarantine` listant les users en quarantaine + bouton « Libérer ».
- Workflow d'appel ban : voir les appels en attente, valider/rejeter (similaire au review modo).
- Lockdown manuel du serveur depuis le web.

---

## 4. 📋 Manques mineurs / nice-to-have (P2)

### 🤝 COM-3. Sponsorships (parrainage)

Endpoints `/api/sponsorships` existent (système de parrainage : nouveau membre + parrain). Pas de page.

**À ajouter** : `/sponsorships` avec liste des couples parrain/filleul, état (pending / validated), récompense XP.

### ⏰ COM-4. Temp roles (rôles temporaires)

Endpoints `/api/temp-roles` existent (assigner un rôle pour X heures, ex. abonnement Premium 30 jours). Pas de page.

**À ajouter** : `/temp-roles` avec liste, ajout manuel, prolongation, expiration.

### 📛 SRV-2. Name history (anti-évasion)

Endpoint `POST /api/name-history` existe (track des changements de pseudo Discord). Pas de page.

**À ajouter** : intégrer dans `/members > Surveillance` (10 derniers pseudos d'un user).

### 🎰 GAME-2. Slot / Wheel — pages dédiées

Le `wheel` est la **signature** du Coude. Aucune page web. Le slot non plus.

**À ajouter** :
- `/wheel` — recent spins (toutes guilds), leaderboard, distribution des cases tombées.
- `/slot` — recent spins, jackpot global, RTP affiché (return to player).

### 🃏 GAME-3. Blackjack tables (multijoueur)

La page `/blackjack` couvre les parties solo. Les **tables multijoueur** (endpoints `/api/blackjack/tables/*`) ne sont pas exposées.

**À ajouter** : section « Tables actives » sur la page blackjack avec liste des tables, joueurs, salons liés.

### 🛠️ SYS-1. Operations / monitoring

Endpoints système non exposés :
- `GET /api/models/status` + `POST /api/models/reload` — IA models monitoring.
- `GET /api/cache/stats` — état des caches Redis.
- `POST /api/ai/jobs` + `GET /api/ai/jobs/{id}` — jobs d'analyse IA en attente/finis.
- `POST /api/exports/jobs` + `GET /api/exports/jobs/{id}` — exports asynchrones.

**À ajouter** : page `/system/operations` regroupant ces métriques techniques.

### 🤖 SYS-2. Bot definitions / status

`/api/bots/definitions` est consommé pour `/component-config`, mais on n'a pas :
- État de chaque bot (running, last seen, version, uptime).
- Health checks.
- Restart à distance d'un bot.

**À ajouter** : sur `/dashboard` ou `/system`, une carte « Bots status » avec badge live/offline.

---

## 5. 🌐 Features Discord-natives non couvertes du tout

Au-delà des features applicatives, un **gestionnaire complet de serveur Discord** devrait aussi exposer :

### 5.1 Gestion des channels

Aujourd'hui : **rien**. Tout passe par Discord directement.

**Manque** :
- Liste des channels (text / voice / forum / stage / category).
- Création / suppression / renommage.
- Édition des permissions par channel.
- Slowmode, NSFW, position, topic.
- Categories management (regrouper les channels).

### 5.2 Webhooks

**Manque** : liste des webhooks par channel, création/suppression, regen URL, audit d'usage.

### 5.3 Server settings (paramètres natifs Discord)

**Manque** :
- Niveau de vérification (None / Low / Medium / High / Highest).
- Filtre de contenu explicite.
- Notifications par défaut (All / Mentions only).
- AFK timeout + AFK channel.
- System messages channel.
- Server icon / banner / splash upload.
- Server description (Discovery).

### 5.4 Discord auto-mod natif

Discord a son propre auto-mod (différent de notre module `automod-bot`). On pourrait l'exposer en lecture / synchronisation.

**Manque** : liste des règles auto-mod natives Discord, état actif/inactif, exemptions.

### 5.5 Onboarding (Server Onboarding Discord 2.0)

Discord a un système d'onboarding officiel (rules screening + interest selection + welcome screen). Non géré.

**Manque** : config du Welcome Screen, des Default Channels, du Server Guide.

### 5.6 Emojis & Stickers

**Manque** : upload, suppression, gestion des emojis/stickers du serveur.

### 5.7 Soundboard (Discord 2024)

**Manque** : liste des sons, upload, suppression.

### 5.8 Threads / Forum / Stage

**Manque** : modération des threads et posts forum, gestion des stages programmés.

### 5.9 Scheduled events (Discord events)

**Manque** : créer / éditer / supprimer les events Discord planifiés (ex. tournoi mensuel Coude → on pourrait le générer auto).

### 5.10 Server boosts / Nitro

**Manque** : voir qui boost, depuis quand, niveau du serveur, stats.

### 5.11 Server insights natifs

Discord fournit des stats officielles (member growth, message activity, retention). On pourrait les pull via API et les afficher.

### 5.12 Server templates

**Manque** : créer un template depuis ton serveur, partager le code, importer un template.

---

## 6. 🎯 Synthèse — priorités pour gérer un serveur de bout en bout

### Tier 1 — Indispensable (à faire en premier)

1. **Welcome / Onboarding** (`/welcome`) — première impression, critique.
2. **Automod page** (`/automod`) — config + logs, sécurité du serveur.
3. **Modération avancée** : evidence + review + strikes + notes + reminders (5 pages ou sections).
4. **Voice channels CRUD complet** (themes, invites, whitelists, transfer).
5. **Role panels CRUD** (création + édition + déploiement depuis web).

### Tier 2 — Important (consolide la suite)

6. **Coude pages avancées** : bounties, coalitions, vendettas, curses (1 page consolidée avec onglets).
7. **Levels config + actions** (XP manuel, multipliers UI).
8. **Channels management** (list / create / edit / delete) — c'est l'élément manquant le plus visible.
9. **Server settings natifs** Discord (verification, content filter, AFK).
10. **Quarantine + Appeal workflow** côté sécurité.

### Tier 3 — Nice-to-have

11. Sponsorships, temp roles, name history (intégrations dans `/members`).
12. Slot / Wheel pages dédiées (analytics jeu).
13. Blackjack tables.
14. System / Operations dashboard.
15. Webhooks management.
16. Emojis & stickers.
17. Scheduled events.
18. Soundboard.
19. Server templates.
20. Server insights natifs.

---

## 7. 📐 Recommandation d'architecture

Avant d'attaquer la longue liste, prévois :

### Refacto SidebarNav

22 pages actuellement, **+30 potentielles** = sidebar à 50 pages → ingérable. Regroupements suggérés :

```
📊 Dashboard
📋 Logs

🛡️ Modération
   ├─ Journal & bans
   ├─ Members
   ├─ Rules
   ├─ Evidence  (nouveau)
   ├─ Reviews   (nouveau)
   ├─ Strikes   (nouveau)
   ├─ Notes     (nouveau)
   ├─ Reminders (nouveau)
   └─ Templates (nouveau)

🔒 Sécurité
   ├─ Events
   ├─ Audit
   ├─ Automod   (nouveau)
   ├─ Quarantine (nouveau)
   └─ Appeals   (nouveau)

🏠 Communauté
   ├─ Welcome   (nouveau)
   ├─ Onboarding (nouveau)
   ├─ Tickets
   ├─ Voice channels
   │  ├─ Active
   │  ├─ Themes (nouveau)
   │  └─ History
   ├─ Role panels
   ├─ Levels
   ├─ Sponsorships (nouveau)
   └─ Temp roles  (nouveau)

🎮 Jeux
   ├─ Games config
   ├─ Coude
   │  ├─ Combats / Stats
   │  ├─ Économie / Wallet
   │  ├─ Bounties / Coalitions / Vendettas (nouveau)
   │  ├─ Curses / Sabotages (nouveau)
   │  └─ Tournaments
   ├─ Blackjack
   │  ├─ Solo games
   │  └─ Tables (nouveau)
   ├─ Slot     (nouveau)
   ├─ Wheel    (nouveau)
   └─ Taunts

🌐 Discord serveur
   ├─ Channels   (nouveau)
   ├─ Roles
   ├─ Webhooks   (nouveau)
   ├─ Emojis     (nouveau)
   ├─ Events     (nouveau)
   └─ Server settings (nouveau)

⚙️ Configuration
   ├─ Component config
   ├─ RBAC
   ├─ Settings
   └─ System / Ops (nouveau)
```

### Patterns à généraliser

- **Sync Discord ↔ Web** : appliquer le pattern décrit dans `SYNC_DISCORD_WEB_DESIGN.md` à toutes les pages (banners, role panels, tickets, etc.).
- **CRUD modal réutilisable** : un composant générique `<EntityModal>` pour create/edit, déjà utilisé partiellement (Discord roles, Games), à étendre.
- **Tabbed pages** : consolider plutôt que multiplier les routes (cf. `/moderation` qui a déjà 2 onglets).
- **Live updates** : SSE généralisé (cf. doc sync) pour ne pas avoir à F5.

---

## 8. 📊 Métrique de complétude

| Domaine | Couverture web actuelle | Couverture cible |
|---|---|---|
| Modération | 60 % | 100 % (manque evidence/review/strikes/notes/reminders) |
| Sécurité | 50 % | 100 % (manque automod/quarantine/appeals) |
| Communauté | 40 % | 100 % (manque welcome/onboarding/sponsorships/temp roles) |
| Jeux Coude | 70 % | 100 % (manque bounties/coalitions/vendettas/curses) |
| Wallet & casinos | 50 % | 100 % (manque slot/wheel/blackjack tables) |
| Configuration | 80 % | 100 % (manque server settings natifs) |
| **Discord natif** | **5 %** | **80 %** (channels, webhooks, emojis, events, server settings) |
| **Total estimé** | **≈ 55 %** | **≈ 95 %** |

---

## 9. 💡 Mon avis

Le projet a **une excellente API** (~120 endpoints non exposés !) et **une web admin solide** mais clairement **inachevée pour la moitié des features**. Les manques se concentrent en 3 zones :

1. **Tout le workflow modération avancé** (evidence, review, strikes, notes, reminders) — l'API est prête, la web brille par son absence. **Tier 1**.
2. **Onboarding & welcome** — le maillon humain le plus visible et le plus négligé. **Tier 1**.
3. **La gestion native Discord** (channels, webhooks, emojis, server settings, events) — terra incognita totale. **Tier 2**, gros chantier.

**Effort total estimé pour atteindre 95 % de couverture** : 2-3 mois à 1 dev temps plein (hors gestion native Discord qui est un projet à part).

---

## 📎 Annexe — Détail des endpoints API non consommés

Synthèse des résultats d'audit (~120 endpoints) :

### Modération
- `GET /modstats/{guild_id}`, `POST /evidence`, `GET /evidence/{action_id}`
- `POST /review`, `GET /review/pending`, `PATCH /review/{id}/resolve`

### Strikes / Notes / Reminders
- 100 % manquants (configs + CRUD)

### Conduct
- `POST /config`, `POST /regen-tick`, `POST /sync-ban-proposals`

### Levels
- `POST /config`, `POST /xp`

### Games (panels)
- `PATCH .../role`, `GET .../by-name/{name}`, `GET .../panels/by-message/{message_id}`, etc.

### Blackjack (tables multi)
- `POST /tables`, `POST /tables/{id}/join`, `GET /tables/{id}/players`, `DELETE /tables/{id}`, `GET /tables/by-channel/{channel_id}`

### Slot / Wheel
- 100 % manquants (`spin`, `recent`, `leaderboard`, `jackpot`)

### Voice channels
- `POST` (create), `PATCH/DELETE` direct, whitelists, bans, invites, themes, transfer, co-admins (~20 endpoints)

### Role panels
- `POST` (create), `DELETE`, `auto-roles` CRUD

### Wallet
- `POST /transfer`, `GET /transactions`

### System / RBAC / AI / Exports
- `models/status`, `cache/stats`, `welcome/{guild_id}`, `ai/jobs`, `exports/jobs`, `rbac/*`

### Bot persistence (intentionnellement bot-only)
- `name-history`, `streak`, `sla`, `sponsorships`, `temp-roles`, `pending` moderation — **OK, pas pour le web**.

---

*Document à valider et prioriser. À mettre à jour au fil des ajouts. Voir aussi `SYNC_DISCORD_WEB_DESIGN.md` pour la synchronisation et `COUDE_ARCHITECTURE_AUDIT.md` pour l'archi backend.*
