# DiscordSentinel — Roadmap

## Priorite 1 : Auto-Roles & Reaction Roles

Assigner des roles automatiquement quand un membre rejoint ou clique sur un bouton/select menu.

**Composants :**
- Bot (nouveau ou extension moderation-bot) : message embed persistant avec boutons/select menu
- API : endpoints CRUD `/api/auto-roles/{guild_id}`
- BDD : table `auto_role_panels` (guild_id, channel_id, message_id, roles mapping JSON)
- Desktop : page de configuration des panels par serveur

**Existant reutilisable :**
- `bot_guild_config` pour stocker la config par serveur
- Infrastructure Tauri/Vue deja en place

---

## Priorite 2 : Systeme de niveaux / XP

Les membres gagnent de l'XP en parlant (texte + vocal) et montent de niveau.

**Composants :**
- Extension stats-bot : calcul XP par message/minute vocale, cooldown anti-spam XP
- Commandes : `/rank [user]`, `/leaderboard [limit]`
- API : endpoints `/api/levels/{guild_id}`, config courbe XP
- BDD : table `level_config` (guild_id, xp_per_message, xp_per_voice_minute, cooldown_secs), table `user_levels` (guild_id, user_id, xp, level)
- Roles-recompenses configurables par palier
- Desktop : page config niveaux + visualisation courbe XP
- Canaux exclus configurables (pas d'XP dans #bot-commands)

**Existant reutilisable :**
- `user_stats` contient deja messages_count et voice_seconds
- Stats-bot track deja l'activite en temps reel

---

## Priorite 3 : Welcome & Goodbye

Messages personnalises d'accueil et de depart.

**Composants :**
- Extension security-bot (ecoute deja les `guild_member_addition`)
- Config par serveur : salon, message embed, variables (`{user}`, `{server}`, `{memberCount}`)
- Option : image de bienvenue generee (canvas)
- API : endpoints `/api/welcome/{guild_id}`
- BDD : table `welcome_config` (guild_id, channel_id, message_template, goodbye_template, enabled)
- Desktop : page de configuration avec preview du message

---

## Priorite 4 : Logs avances (Audit Log)

Logger les actions serveur Discord en temps reel dans un salon dedie.

**Evenements a capturer :**
- Modifications de roles/permissions
- Suppression/edition de messages
- Entrees/sorties vocales
- Creations/suppressions de salons
- Changements de pseudo/avatar
- Bans/kicks manuels Discord (hors Sentinel)

**Composants :**
- Nouveau bot `audit-bot` ou extension du security-bot
- Config par serveur : salon de log, evenements actives/desactives
- API : endpoints `/api/audit/{guild_id}`
- BDD : table `audit_logs` (guild_id, event_type, actor_id, target_id, details JSONB, created_at)
- Desktop : page de consultation avec filtres par type/acteur/cible

**Existant reutilisable :**
- Table `logs` existe deja, peut etre etendue ou remplacee
- WebSocket broadcaster pour le temps reel

---

## Priorite 5 : Page Membres

Gestion et consultation des membres depuis le desktop.

**Composants :**
- API : endpoints `/api/members/{guild_id}`, `/api/members/{guild_id}/{user_id}`
- BDD : table `guild_members` (guild_id, user_id, username, avatar, roles JSON, joined_at, last_active_at)
- Desktop : `MembersPage.vue` avec liste, recherche, filtres par role
- Fiche membre detaillee : roles, infractions, conduite, activite, evenements securite

**Existant reutilisable :**
- Design deja redige dans `docs/gestion-utilisateurs.md`
- `WatchedUsersPage` comme modele de fiche utilisateur
- Donnees deja presentes dans infractions, moderation_actions, user_stats, conduct_points

---

## Priorite 6 : Anti-Phishing / Anti-Scam

Detection de liens de phishing et messages scam.

**Composants :**
- Extension automod-bot : nouveau detecteur `detectors/phishing.rs`
- Sources : API Phisherman, Sinking Yachts, ou base locale de domaines malveillants
- Detection de patterns scam connus ("Free Nitro", "Steam gift", etc.)
- Action configurable : delete + warn, ou delete + mute
- API : nouveau flag_type `phishing` dans le systeme de scoring

**Existant reutilisable :**
- Architecture detecteurs automod-bot (spam, insult, link)
- Systeme de scoring/rules deja en place
- Il suffit d'ajouter un nouveau FlagType

---

## Priorite 7 : Escalade automatique des sanctions

Sanctions progressives : warn -> mute -> ban temp -> ban permanent.

**Composants :**
- API : config escalade par serveur (seuils, durees, reset)
- BDD : table `escalation_config` (guild_id, warns_before_mute, mutes_before_ban, reset_after_days)
- Integration dans `AnalyzeMessageService` : verifier l'historique avant de decider l'action
- Desktop : page de configuration de l'escalade

**Existant reutilisable :**
- Systeme de conduite (points) fait deja une partie du travail
- `moderation_actions` et `infractions` contiennent l'historique complet

---

## Priorite 8 : Messages programmes / Annonces

Programmer des messages a envoyer a une date/heure precise.

**Composants :**
- API : endpoints CRUD `/api/scheduled-messages/{guild_id}`
- BDD : table `scheduled_messages` (guild_id, channel_id, content, embed JSON, send_at, recurrence, status)
- Worker : tache cron qui envoie les messages a l'heure prevue
- Desktop : page calendrier avec creation/edition/suppression

**Existant reutilisable :**
- Worker service prevu dans l'architecture (placeholder `services/worker/`)
- Tokio spawn pour les taches planifiees (deja utilise pour conduct regen)

---

## Priorite 9 : Commandes personnalisees

Creer des commandes texte simples depuis le desktop.

**Composants :**
- API : endpoints CRUD `/api/custom-commands/{guild_id}`
- BDD : table `custom_commands` (guild_id, trigger, response_text, response_embed JSON, enabled)
- Bot : listener sur les messages qui match les triggers
- Desktop : page de gestion avec editeur de commandes
- Support variables : `{user}`, `{server}`, `{channel}`, `{date}`

---

## Priorite 10 : Backup & Restore

Sauvegarder la configuration complete du serveur.

**Composants :**
- API : endpoints `POST /api/backup/{guild_id}`, `POST /api/restore/{guild_id}`
- Export : rules, conduct_config, bot_guild_config, escalation_config, welcome_config, auto_roles
- Format : JSON structure
- Desktop : page backup avec historique, telechargement, restauration partielle

---

## Nice to have

### Giveaways
Systeme de concours : duree, conditions (role requis, anciennete), tirage au sort automatique.
- Commande `/giveaway create <prize> <duration> [role_required]`
- Message embed avec bouton de participation
- Timer + tirage + annonce du gagnant

### Sondages avances
Sondages multi-options, duree, resultats temps reel, anonymat optionnel.
- Commande `/poll <question> <options...> [duration]`
- Boutons de vote avec compteur

### Starboard
Messages avec X reactions etoile automatiquement postes dans un salon "hall of fame".
- Config : salon starboard, seuil de reactions, emoji configurable

### Tableau de bord temps reel
Graphiques d'activite dans le desktop : messages/heure, membres actifs, croissance 7/30 jours.
- La data existe deja dans `user_stats`
- Librairie de graphiques (Chart.js ou equivalent)

### Integration Twitch / YouTube
Notifications quand un streamer passe en live, role automatique pour les subs.
- Webhook Twitch/YouTube → bot notification
- Config par serveur : streamers suivis, salon de notification
