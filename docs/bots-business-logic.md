# Bots Discord Sentinel — Regles Metier

Ce document decrit la logique metier exacte de chaque bot, telle qu'implementee dans le code source.

---

## Renommages prevus

Deux bots vont etre renommes pour refleter leur perimetre reel apres les extensions planifiees :

| Ancien nom | Nouveau nom | Raison |
|-----------|------------|--------|
| `stats-bot` | **`progression-bot`** | Ne fait plus seulement des "stats" : gere l'XP, les niveaux, les streaks, les badges, les cooldowns, les multiplicateurs. "Progression" reflete le parcours du membre dans le serveur. |
| `roles-bot` | **`community-bot`** | Ne fait plus seulement des "roles" : gere les auto-roles, les panels, l'onboarding des nouveaux membres, le parcours guide, le tracking de retention. "Community" reflete la mission d'accueil et d'integration. |

### Impact du renommage

Pour chaque bot renomme, les fichiers/references suivants doivent etre mis a jour :

**`stats-bot` → `progression-bot` :**

| Element | Ancien | Nouveau |
|---------|--------|---------|
| Dossier | `bots/stats-bot/` | `bots/progression-bot/` |
| Cargo.toml name | `sentinel-stats-bot` | `sentinel-progression-bot` |
| Variable env token | `STATS_DISCORD_TOKEN` | `PROGRESSION_DISCORD_TOKEN` |
| Heartbeat name | `"stats-bot"` | `"progression-bot"` |
| Docker service | `stats-bot` | `progression-bot` |
| bot_definitions (BDD) | `stats-bot` | `progression-bot` |
| Dockerfile binary | `sentinel-stats-bot` | `sentinel-progression-bot` |
| Tracing filter | `sentinel_stats_bot` | `sentinel_progression_bot` |
| Logs (category bot) | `"stats-bot"` | `"progression-bot"` |

**`roles-bot` → `community-bot` :**

| Element | Ancien | Nouveau |
|---------|--------|---------|
| Dossier | `bots/roles-bot/` | `bots/community-bot/` |
| Cargo.toml name | `sentinel-roles-bot` | `sentinel-community-bot` |
| Variable env token | `ROLES_DISCORD_TOKEN` | `COMMUNITY_DISCORD_TOKEN` |
| Heartbeat name | `"roles-bot"` | `"community-bot"` |
| Docker service | `roles-bot` | `community-bot` |
| bot_definitions (BDD) | `roles-bot` | `community-bot` |
| Dockerfile binary | `sentinel-roles-bot` | `sentinel-community-bot` |
| Tracing filter | `sentinel_roles_bot` | `sentinel_community_bot` |
| Logs (category bot) | `"roles-bot"` | `"community-bot"` |

**Fichiers concernes en dehors des bots :**
- `docker-compose.yml` — renommer les services
- `.env.example` — renommer les variables
- `README.md` — mettre a jour les references
- `services/api/migrations/` — migration pour mettre a jour `bot_definitions` et `bot_guild_config`
- `apps/desktop/src-tauri/src/` — si des references hardcodees existent
- `docs/bots-business-logic.md` — ce document (sections 4 et 9)

---

## 1. Automod Bot — Auto-moderation des messages

### Evenements Discord ecoutes

- `message` (intents: GUILD_MESSAGES, MESSAGE_CONTENT)

### Pipeline de traitement d'un message

```
Message recu
  |
  v
1. Deduplication (DashSet de MessageId)
  |
2. Chargement config per-guild (GET /guild-config/{guild_id})
  |
3. Filtrage : ignore si canal ou role exclu
  |
4. Detection flood locale
  |   - Tracking (channel_id, user_id) avec timestamps
  |   - Seuil : 5 messages en 10 secondes (configurable)
  |   - Si flood : warn + envoyer au backend comme "spam"
  |
5. Detection majuscules
  |   - Message >= 8 caracteres, 100% majuscules
  |   - Avertissement local uniquement
  |
6. Analyse locale du contenu (4 detecteurs)
  |   +-- spam : repetition de caracteres (>=6) ou de mots (>=5 fois le meme)
  |   +-- insult : dictionnaire regex FR/EN (~50 patterns)
  |   +-- link : URLs http/https, discord.gg, discord.com/invite
  |   +-- phishing : typosquatting Discord/Steam/crypto, IP grabbers, scam patterns
  |
7. Si flags detectes → POST /analyze (scoring backend + inference IA)
  |   - Backend retourne : action (none/warn/delete/mute/ban) + raison + score
  |
8. Execution de l'action
      +-- warn : reponse dans le canal avec la raison
      +-- delete : avertissement puis suppression du message
      +-- mute : avertissement, suppression, timeout Discord (disable_communication_until)
      +-- ban : avertissement, suppression, enregistrement comme "proposition de ban" (PAS de ban execute)
```

### Fallback si API indisponible

- Phishing detecte : suppression + avertissement
- Insulte detectee : suppression + avertissement
- Autre : aucune action

### Configuration per-guild

| Parametre | Defaut | Description |
|-----------|--------|-------------|
| `flood_max_messages` | 5 | Messages max dans la fenetre |
| `flood_window_secs` | 10 | Fenetre de detection flood (secondes) |
| `mute_duration_secs` | 600 | Duree du mute (secondes) |
| `ignored_channels` | [] | Canaux exclus de la moderation |
| `ignored_roles` | [] | Roles exclus de la moderation |
| `log_channel_id` | null | Canal pour les logs rich embed |

---

## 2. Moderation Bot — Actions manuelles des moderateurs

### Evenements Discord ecoutes

- `interaction_create` (slash commands + boutons)

### Commandes slash

#### `/warn <user> <gravity> <reason>`

1. Envoie l'action au backend : POST `/api/moderation/actions`
2. DM a l'utilisateur avec la severite (emoji) et le nom du serveur
3. Reponse dans le canal avec le resume de l'avertissement

Gravites :
- `low` (🟡) | `medium` (🟠) | `high` (🔴)

#### `/mute <user> <reason> [duration_minutes]`

1. Lit la config guild : `default_mute_duration_secs`, `max_mute_duration_secs`
2. Si pas de duree : utilise `default_mute_duration_secs`
3. Plafonne a 28 jours (max Discord)
4. Applique le timeout Discord : `disable_communication_until_datetime()`
5. DM a l'utilisateur
6. Log au backend : type `mute_permanent` ou `mute_temp` selon la duree

#### `/unmute <user>`

1. Appelle `enable_communication()` sur le membre
2. Log au backend : type `unmute`

#### `/ban <user> <reason> [duration_hours]`

1. Lit la config : `ban_delete_message_days` (defaut: 1 jour)
2. DM a l'utilisateur AVANT le ban (impossible apres)
3. Execute le ban Discord avec suppression des messages
4. Log au backend : type `ban_permanent` ou `ban_temp`

#### `/unban <user_id>`

1. Appelle `guild.unban(user_id)`
2. Log au backend : type `unban`

#### `/history <user>`

1. GET `/api/moderation/history/{guild_id}/{user_id}`
2. Affiche : total warns/mutes/bans + 10 dernières actions
3. Reponse ephemere (visible seulement par le moderateur)

### Fallback si API indisponible

L'action Discord est toujours executee. Seul le log backend est ignore.

---

## 3. Security Bot — Protection du serveur

### Evenements Discord ecoutes

- `guild_member_addition` (nouveau membre)
- `guild_member_removal` (depart)
- `channel_create`, `channel_delete`
- `guild_ban_addition`, `guild_ban_removal`
- `interaction_create` (bouton captcha)

### Anti-Raid

```
Nouveau membre rejoint
  |
  v
RaidDetector : compteur de joins par guild dans une fenetre glissante
  |
  Si joins >= RAID_JOIN_THRESHOLD dans RAID_JOIN_WINDOW_SECS :
  |
  +-- POST /api/security/events (event_type: "raid_detected")
  +-- Passer la verification du serveur au niveau "Highest"
  +-- Activer le slowmode sur tous les canaux texte (si configure)
  +-- Quarantaine + captcha sur les nouveaux arrivants (si active)
  +-- Annonce dans le premier canal texte
  +-- Reset du detecteur
```

### Verification age de compte

```
Nouveau membre rejoint
  |
  v
AccountChecker : compare user.created_at vs maintenant
  |
  Si compte < MIN_ACCOUNT_AGE_SECS (defaut: 24h) :
  |
  +-- POST /api/security/events (event_type: "suspicious_account")
  +-- Quarantaine + captcha (si active)
```

### Systeme de quarantaine

- Assigne un role restrictif au membre suspect
- L'utilisateur recoit un DM avec un bouton "Je suis humain"
- Tache de fond (toutes les 30s) : kick les utilisateurs qui n'ont pas repondu au captcha apres `CAPTCHA_TIMEOUT_SECS` (defaut: 5 min)

### Slowmode automatique

- Active le slowmode sur tous les canaux texte pendant un raid
- Sauvegarde les valeurs precedentes pour les restaurer
- Tache de fond (toutes les 15s) : restaure le slowmode apres expiration (`SLOWMODE_DURATION_SECS`)

### Configuration

| Parametre | Defaut | Description |
|-----------|--------|-------------|
| `RAID_JOIN_THRESHOLD` | 10 | Joins pour declencher l'alerte raid |
| `RAID_JOIN_WINDOW_SECS` | 10 | Fenetre de detection (secondes) |
| `MIN_ACCOUNT_AGE_SECS` | 86400 | Age minimum du compte (1 jour) |
| `QUARANTINE_ROLE_ID` | - | ID du role de quarantaine |
| `QUARANTINE_ENABLED` | false | Active la quarantaine |
| `CAPTCHA_ENABLED` | false | Active le captcha DM |
| `CAPTCHA_TIMEOUT_SECS` | 300 | Timeout captcha avant kick |
| `SLOWMODE_SECONDS` | 10 | Valeur du slowmode pendant un raid |
| `SLOWMODE_DURATION_SECS` | 300 | Duree du slowmode automatique |

---

## 4. Progression Bot (ex Stats Bot) — Statistiques, XP et progression

### Evenements Discord ecoutes

- `message` (messages non-bot)
- `voice_state_update` (join/leave vocal)
- `interaction_create` (slash commands)

### Tracking des messages

```
Message recu (non-bot)
  |
  +-- Cache local : StatsTracker.record_message()
  +-- POST /api/stats/messages (guild_id, user_id, username, count=1)
  +-- POST /api/levels/xp (guild_id, user_id, username, amount=15)
       |
       Si level up dans la reponse :
         +-- Annonce dans le canal : "GG <@user>, tu es maintenant niveau X!"
         +-- Assigne le role de recompense si configure
```

### Tracking vocal

```
Voice state update
  |
  Join : enregistre le timestamp dans voice_sessions
  |
  Leave : calcule la duree
    |
    +-- POST /api/stats/voice (guild_id, user_id, username, seconds)
    +-- POST /api/levels/xp (amount = (seconds / 60) * 5)
```

### Commandes slash

| Commande | Description | API |
|----------|-------------|-----|
| `/stats user [target]` | Stats d'un utilisateur (messages, vocal, infractions) | GET /api/stats/.../user/... + GET /infractions/... |
| `/stats server` | Stats globales du serveur | GET /api/stats/.../overview |
| `/stats top [limit]` | Classement (max 25) | GET /api/stats/.../leaderboard |
| `/level [user]` | Niveau et XP (barre de progression) | GET /api/levels/.../... |
| `/level top [limit]` | Classement niveaux | GET /api/levels/.../leaderboard |

### XP par action

| Action | XP |
|--------|-----|
| Message envoye | 15 XP |
| Minute en vocal | 5 XP |

### Fallback

Cache local `StatsTracker` utilise si l'API est indisponible.

---

## 5. Ticket Bot — Support par tickets

### Evenements Discord ecoutes

- `interaction_create` (boutons, menus, modals)
- `message` (messages dans les canaux ticket-*)
- `ready` (deploiement du panel)

### Cycle de vie d'un ticket

```
1. CREATION
   Panel dans le canal d'assistance → bouton "Creer un ticket"
     |
     v
   Menu de selection du type (7 types)
     |
     v
   Modal : titre + priorite
     |
     v
   POST /api/tickets → cree le ticket backend
   Cree le canal Discord "ticket-{id}" (prive : createur + staff)

2. CONVERSATION
   Chaque message dans ticket-* :
     → POST /api/tickets/{id}/messages (author_role: user ou moderator)

   Reponses depuis le desktop app :
     → Redis pub/sub → message poste dans le canal Discord

3. ASSIGNATION
   Bouton "Inviter" → ajoute un moderateur au canal
   PATCH /api/tickets/{id}/assign

4. APPEL VOCAL
   Bouton "Appel vocal" → cree un canal vocal temporaire
   PATCH /api/tickets/{id}/channels

5. FERMETURE
   Bouton "Fermer" → modal de confirmation
   PATCH /api/tickets/{id}/close
   Suppression du canal Discord
```

### Types de tickets

| Type | Description | Particularite |
|------|-------------|---------------|
| `probleme_serveur` | Probleme avec le serveur | - |
| `probleme_membre` | Probleme avec un membre | - |
| `probleme_moderateur` | Probleme avec un moderateur | Visibilite restreinte (admin only) |
| `appel_sanction` | Contester une sanction | - |
| `urgence_detresse` | Situation d'urgence | Priorite auto: urgent |
| `question` | Question generale | - |
| `autre` | Autre sujet | - |

### Auto-fermeture

Tache de fond (toutes les 30 min) : ferme les tickets inactifs depuis 7 jours.

### Synchronisation temps reel

Redis pub/sub (`sentinel:events`) : les reponses des moderateurs depuis l'app desktop sont relayees dans le canal Discord.

---

## 6. Image Bot — Detection d'images IA

### Evenements Discord ecoutes

- `message` (intents: GUILD_MESSAGES, MESSAGE_CONTENT)

### Pipeline de traitement

```
Message avec piece jointe ou embed image
  |
  v
1. Detection des images :
   - Attachments : filtre par MIME type ou extension
   - Embeds : image + thumbnail
   - Extensions : jpg, jpeg, png, gif, webp, bmp
  |
2. Deduplication (DashSet de MessageId)
  |
3. Telechargement de l'image
   - Verifie la taille < MAX_IMAGE_SIZE (defaut: 10 Mo)
  |
4. Detection du content type par magic bytes :
   - JPEG : 0xFF 0xD8 0xFF
   - PNG : 0x89 0x50 0x4E 0x47
   - GIF : "GIF8"
   - WEBP : "RIFF" + "WEBP" a l'offset 8
   - Fallback : extension
  |
5. Encodage base64
  |
6. POST /analyze/image (guild_id, channel_id, user_id, image_data, content_type)
   - Backend : inference ONNX EfficientNetV2-S
   - Classifications : safe / nsfw / illicit
  |
7. Execution de l'action selon le score :
   - warn : avertissement
   - delete : avertissement + suppression
   - mute : avertissement + suppression + timeout 10 min
   - ban : avertissement + suppression + ban (1 jour de messages supprimes)
```

### Fallback si API indisponible

**Suppression preventive** de l'image + avertissement "verification impossible". C'est le seul bot avec un fallback aussi agressif — par securite.

---

## 7. Voice Bot — Salons vocaux dynamiques

### Evenements Discord ecoutes

- `voice_state_update` (join/leave vocal)
- `interaction_create` (boutons, modals, menus)
- `message` (commandes texte dans les canaux admin)

### Fonctionnalites

#### Creation de salon

- L'utilisateur rejoint un canal "hub" (configurable)
- Le bot cree automatiquement un salon vocal temporaire
- Le createur est proprietaire : controle total des permissions
- Le salon est supprime quand tous les membres partent

#### Panneau d'administration

- Panneau embed dans un canal texte lie au salon vocal
- Boutons : renommer, fermer, transferer, kick, inviter, verrouiller

#### Systeme de co-admins

- Le proprietaire peut ajouter des co-admins
- Les co-admins ont les memes droits que le proprietaire (sauf transfert)

#### Vote kick

- Un membre peut initier un vote pour ejecter quelqu'un
- Necessite une majorite (>50% des presents)
- VoteTracker : suivi des votes en cours avec timeout

#### Whitelist / Ban

- Le proprietaire peut whitelister ou bannir des utilisateurs
- Les bans vocaux peuvent avoir une expiration

#### File d'attente

- Si le salon est plein, les utilisateurs sont mis en file d'attente
- Deplacements automatiques quand une place se libere

#### Anti-flood

- CooldownTracker : empeche le spam de creation de salons
- FloodTracker : empeche les interactions en rafale

### Etat persistant

| Map | Cle → Valeur |
|-----|-------------|
| TextToVoiceMap | text_channel → voice_channel |
| MembersToVoiceMap | members_channel → voice_channel |
| VoiceOwnerMap | voice_channel → owner_user_id |

Charge au demarrage depuis l'API.

---

## 8. Audit Bot — Journal d'audit complet

### Evenements Discord ecoutes

Tous les evenements majeurs :

| Categorie | Evenements |
|-----------|-----------|
| Messages | `message_delete`, `message_update`, `message_delete_bulk` |
| Membres | `guild_member_addition`, `guild_member_removal`, `guild_member_update` (roles, pseudo) |
| Moderation | `guild_ban_addition`, `guild_ban_removal` |
| Roles | `guild_role_create`, `guild_role_delete`, `guild_role_update` |
| Canaux | `channel_create`, `channel_delete`, `channel_update` |
| Vocal | `voice_state_update` (joins, leaves, moves) |
| Invitations | `invite_create`, `invite_delete` |
| Threads | `thread_create`, `thread_delete`, `thread_update` |

### Format de l'evenement

Chaque evenement genere un `AuditEvent` envoye au backend :

```
POST /api/audit-logs
{
  "guild_id": "...",
  "event_type": "member_ban",        // type normalise
  "actor_id": "...",                  // qui a fait l'action
  "actor_name": "...",
  "target_id": "...",                 // qui est affecte
  "target_name": "...",
  "channel_id": "...",               // ou ca s'est passe
  "channel_name": "...",
  "details": { ... }                 // donnees specifiques a l'evenement
}
```

### Details par type d'evenement

| Type | Details enregistres |
|------|-------------------|
| `message_delete` | content (si disponible), author_id |
| `message_update` | old_content, new_content |
| `member_join` | account_age_days |
| `member_leave` | joined_at, roles |
| `member_ban` | reason |
| `member_roles_update` | old_roles, new_roles |
| `voice_join/leave/move` | channel_name, from_channel, to_channel |
| `role_create/update` | name, color, permissions |
| `channel_create/delete` | name, type |

---

## 9. Community Bot (ex Roles Bot) — Roles, onboarding et communaute

### Evenements Discord ecoutes

- `guild_member_addition` (attribution auto-role)
- `interaction_create` (clic sur bouton role)

### Auto-roles a l'arrivee

```
Nouveau membre rejoint
  |
  v
GET /api/auto-roles/{guild_id}
  |
  Pour chaque auto_role active :
    |
    Si delay_secs > 0 :
      → Tache async : attendre delay_secs puis add_role
    Sinon :
      → add_role immediatement
```

### Panels de roles

#### Deploiement

```
/roles-panel deploy <panel_id>
  |
  v
GET /api/role-panels/detail/{panel_id}
  |
  v
Construire l'embed + boutons (max 5 par ligne)
  - Bouton custom_id = "role_{role_id}"
  - Emoji + label configurables
  |
  v
Envoyer le message dans le canal courant
PATCH /api/role-panels/set-message (sauvegarder le message_id)
```

#### Clic sur un bouton role

```
Interaction composant "role_*"
  |
  v
Extraire role_id du custom_id
  |
  v
Le membre a deja le role ?
  +-- Oui : retirer le role → "Role retire!"
  +-- Non : ajouter le role → "Role attribue!"
  |
  v
Reponse ephemere
```

### Commandes slash

| Commande | Description |
|----------|-------------|
| `/roles-panel deploy <panel_id>` | Deploie un panel dans le canal courant |
| `/roles-panel list` | Liste les panels disponibles pour ce serveur |

---

## Resume : Comportement de fallback

| Bot | Comportement si API indisponible |
|-----|--------------------------------|
| **Automod** | Suppression locale pour phishing/insultes, sinon rien |
| **Moderation** | Actions Discord executees, log backend ignore |
| **Security** | Detection locale fonctionne, events non envoyes |
| **Progression** (ex Stats) | Cache local StatsTracker utilise |
| **Ticket** | Operations Discord fonctionnent, sync backend ignore |
| **Image** | **Suppression preventive** de l'image (fail-safe) |
| **Voice** | Evenements vocaux traites localement |
| **Community** (ex Roles) | Attribution de roles fonctionne localement |
| **Audit** | Evenements logues localement, envoyes quand l'API revient |

---

## Idees de logiques metier a implementer

### Automod Bot

| Feature | Description | Priorite |
|---------|-------------|----------|
| **Detection de toxicite contextuelle** | Utiliser l'inference IA text cote bot (avant envoi API) pour pre-filtrer les messages tres toxiques sans latence reseau. Score local rapide + score backend approfondi | HIGH |
| **Escalade progressive** | Systeme de strikes : 1er warn → 2e warn → mute auto → ban auto. Tracking local du nombre de warns recents par utilisateur avec fenetre glissante (ex: 3 warns en 1h = mute auto) | HIGH |
| **Detection de raid textuel** | Detecter quand plusieurs comptes postent le meme message/lien simultanement (copier-coller coordonne). Hashing du contenu + comparaison entre utilisateurs dans une fenetre de 30s | HIGH |
| **Mode nuit** | Regles de moderation plus strictes entre certaines heures (ex: 23h-7h). Seuils abaisses, actions plus severes. Configurable per-guild | MEDIUM |
| **Anti-mention mass** | Detecter les messages avec trop de @mentions (seuil configurable, ex: >5 mentions = flag). Protege contre le spam de mentions et le harcelement cible | MEDIUM |
| **Detection unicode abuse** | Detecter les caracteres zalgo, invisible characters, lookalike unicode (ex: "а" cyrillique au lieu de "a" latin) utilises pour contourner les filtres | MEDIUM |
| **Whitelist de liens** | Liste de domaines autorises per-guild. Les liens vers des domaines non-whitelistes sont flag "link" meme sans pattern phishing | LOW |
| **Auto-purge** | Commande `/purge <count> [user]` : suppression en masse de messages. Utile apres un raid textuel. Log chaque message supprime au backend | LOW |
| **Analyse des images textuelles** | OCR leger pour detecter du texte toxique dans les images (screenshots de messages haineux). Envoi du texte extrait a /analyze | LOW |
| **Slowmode intelligent adaptatif** | Au lieu d'un slowmode fixe on/off (raid uniquement), adapter automatiquement le slowmode d'un canal selon l'activite en temps reel. Beaucoup de messages = slowmode monte progressivement (5s→10s→30s), retour au calme = slowmode baisse. Anti-flood continu sans intervention manuelle. Seuils configurables per-guild per-canal | MEDIUM |

### Moderation Bot

| Feature | Description | Priorite |
|---------|-------------|----------|
| **Sanctions temporaires avec rappel** | Programmer un rappel DM au moderateur X heures avant l'expiration d'un mute/ban temporaire, pour qu'il decide de prolonger ou non | HIGH |
| **Moderation en masse** | `/massmute <role> <reason> [duration]` et `/massban <role> <reason>` : actions sur tous les membres d'un role (utile post-raid). Avec confirmation obligatoire et log detaille | HIGH |
| **Systeme de notes** | `/note <user> <text>` : ajouter une note interne sur un utilisateur sans action disciplinaire. Visible dans `/history`. Permet aux moderateurs de documenter des comportements suspects avant d'agir | MEDIUM |
| **Raisons predefinies** | Autocomplete sur le champ raison avec des templates configurables per-guild (ex: "Spam repetitif", "Contenu NSFW", "Harcelement"). Accelere la moderation | MEDIUM |
| **Moderation programmee** | `/schedule-mute <user> <datetime> <duration> <reason>` : programmer une action future. Utile pour "si tu recommences dans les 24h, c'est mute" | MEDIUM |
| **Convocation (`/call`)** | `/call <user> [raison]` : le moderateur convoque un utilisateur dans un salon textuel prive cree automatiquement. Seuls le moderateur, l'utilisateur cible et les admins peuvent voir le canal. Permet de discuter avant de sanctionner, de demander des explications, ou de medier un conflit entre membres. Le canal est supprime a la fermeture (bouton ou commande `/call close`). Log de la conversation sauvegarde en BDD. Historique des convocations visible dans `/history` | HIGH |
| **Mode apprenti moderateur** | Role "apprenti mod" qui peut proposer des actions (warn/mute) via `/propose-warn`, `/propose-mute`. La proposition est envoyee aux mods seniors dans un canal dedie avec boutons Accepter/Refuser. Si acceptee, l'action est executee automatiquement. Permet de former les nouveaux moderateurs sans risque d'abus. Stats de propositions acceptees/refusees par apprenti | HIGH |
| **Commande `/context`** | `/context <message_id>` : affiche les 10 messages avant et 10 apres un message signale, dans un embed prive. Aide les moderateurs a comprendre le contexte d'une infraction sans scroller dans le canal. Utile quand un message est signale hors contexte | MEDIUM |
| **Appel de sanction** | `/appeal <sanction_id>` : l'utilisateur peut contester une sanction. Cree automatiquement un ticket avec le contexte (infraction, preuves, historique) | LOW |
| **Export historique** | `/export-history <user>` : genere un fichier CSV/JSON avec tout l'historique de moderation d'un utilisateur. Utile pour les rapports a Discord Trust & Safety | LOW |

### Security Bot

| Feature | Description | Priorite |
|---------|-------------|----------|
| **Detection de raid par pattern** | En plus du volume de joins, detecter les patterns : memes noms similaires (regex), memes avatars (hash), comptes crees a la meme heure. Score de suspicion composite | HIGH |
| **Lockdown automatique** | Quand un raid est detecte, verrouiller automatiquement les canaux (deny send_messages pour @everyone). Restauration automatique apres X minutes ou commande manuelle | HIGH |
| **Captcha intelligent** | Au lieu d'un simple bouton, poser une question simple (ex: "Combien font 3+5?", "Quel est le nom de ce serveur?"). Plus difficile a bypass pour les selfbots | MEDIUM |
| **Verification par reaction** | Alternative au captcha DM : poster un message dans un canal de verification avec un emoji a cliquer. Plus intuitif pour les utilisateurs mobiles | MEDIUM |
| **Honeypot channels** | Creer des canaux invisibles aux vrais utilisateurs mais visibles aux bots de raid (via permissions). Si quelqu'un poste dedans → ban automatique + log | MEDIUM |
| **IP reputation check** | Integration avec des APIs de reputation IP (AbuseIPDB, etc.) via le proxy de l'API backend. Flag les connexions depuis des VPN/proxies connus | LOW |
| **Whitelist de serveurs** | Verifier si le nouveau membre est aussi dans des serveurs "de confiance" (mutual guilds). Les membres de serveurs whitelistes passent directement | LOW |
| **Dashboard raid en temps reel** | Envoyer des metriques detaillees via WebSocket pendant un raid : joins/sec, comptes suspects, actions prises. Visible dans l'app desktop | LOW |
| **Detection de comptes alts** | Quand un membre banni revient avec un autre compte, detecter via patterns : memes premiers messages, pseudo similaire (Levenshtein), timing de creation proche du ban, comportement identique (memes canaux, memes heures). Score de suspicion composite. Alerte aux modos avec les deux profils compares cote a cote | HIGH |
| **Blacklist partagee entre serveurs** | Les admins de serveurs partenaires partagent une blacklist d'utilisateurs problematiques via l'API backend. Un ban sur un serveur = alerte automatique sur les serveurs partenaires. Opt-in par guild, avec validation manuelle avant action. Dashboard des blacklists partagees dans l'app desktop | MEDIUM |

### Progression Bot (ex Stats Bot)

| Feature | Description | Priorite |
|---------|-------------|----------|
| **Streaks et multiplicateurs XP** | Bonus XP pour les jours consecutifs d'activite (streak). x1.5 apres 3 jours, x2 apres 7 jours, x3 apres 30 jours. Reset si 24h sans activite | HIGH |
| **Cooldown XP par message** | Empecher le farming d'XP par spam. Cooldown de 60s entre les gains d'XP par message (un seul gain par minute). Deja 15 XP/msg mais sans cooldown | HIGH |
| **Boosts XP par canal/role** | Canaux ou roles avec multiplicateur XP configurable (ex: #aide x2, role "Actif" x1.5). Per-guild via la config | MEDIUM |
| **Weekly/monthly recap** | DM hebdomadaire ou mensuel optionnel aux utilisateurs avec leurs stats : messages, vocal, XP gagnes, progression de niveau, rank dans le serveur | MEDIUM |
| **Statistiques de retention** | Tracker les membres qui partent et quand (apres combien de jours, quel etait leur niveau). Identifier les patterns de depart | MEDIUM |
| **Heatmap d'activite personnelle** | `/stats heatmap [user]` : affiche un heatmap de l'activite par heure/jour de la semaine. Embed avec blocs colores | LOW |
| **Comparaison entre utilisateurs** | `/stats compare <user1> <user2>` : compare les stats de deux utilisateurs cote a cote | LOW |
| **Achievements/badges** | Systeme de badges pour des accomplissements : "Premier message", "100h de vocal", "Niveau 50", "1000 messages". Affichables dans le profil | LOW |
| **Reputation entre membres** | Les membres peuvent `/thank @user` ou `/vouch @user` pour donner des points de reputation (+1 par jour max par membre). Visible dans `/stats user` et dans le profil. Les moderateurs voient qui est bien vu par la communaute vs qui est isole (indicateur de risque dans le dossier WatchedUsers). Anti-abuse : cooldown 24h par paire, pas de self-vouch, minimum niveau 5 pour voter | MEDIUM |

### Ticket Bot

| Feature | Description | Priorite |
|---------|-------------|----------|
| **SLA et temps de reponse** | Tracker le temps de premiere reponse et le temps de resolution par ticket. Alerter si un ticket depasse le SLA (ex: pas de reponse apres 2h pour priorite urgent) | HIGH |
| **Satisfaction post-fermeture** | Apres fermeture, DM au createur avec un sondage (1-5 etoiles + commentaire optionnel). Stocker le feedback et afficher les stats dans le dashboard | HIGH |
| **Templates de reponse** | Reponses predefinies pour les moderateurs (/template list, /template use <id>). Accelere le traitement des tickets recurrents | MEDIUM |
| **Escalade automatique** | Si un ticket reste sans reponse pendant X minutes, escalader la priorite et notifier un role superieur | MEDIUM |
| **Transcript** | A la fermeture du ticket, generer un transcript HTML/Markdown complet du thread et l'archiver (stockage ou envoi dans un canal d'archives) | MEDIUM |
| **Ticket par DM** | Permettre aux utilisateurs d'ouvrir un ticket en DM au bot (pour ceux qui n'osent pas dans le serveur). Le bot cree le canal prive automatiquement | LOW |
| **Tags et categories** | Systeme de tags sur les tickets (ex: #bug, #urgent, #resolved). Filtrable dans le dashboard | LOW |
| **FAQ automatique** | Avant de creer un ticket, proposer des reponses FAQ basees sur le titre saisi (recherche par mots-cles). Reduit le nombre de tickets inutiles | LOW |

### Image Bot

| Feature | Description | Priorite |
|---------|-------------|----------|
| **Cache de hashes** | Calculer un hash perceptuel (pHash) de chaque image analysee. Si une image deja flag est repostee (meme avec modifications mineures), detection instantanee sans appel API | HIGH |
| **Detection de stickers/emojis custom** | Scanner les stickers et emojis custom ajoutes au serveur pour du contenu NSFW. Verification a l'ajout + scan periodique | MEDIUM |
| **Seuils adaptatifs par canal** | Certains canaux tolerent plus de contenu (ex: #art, #memes). Seuils NSFW configurables per-channel, pas seulement per-guild | MEDIUM |
| **OCR + analyse texte** | Extraire le texte des images (screenshots) et le passer a l'analyse de sentiment. Detecte le contenu haineux dans les captures d'ecran | MEDIUM |
| **Detection GIF anime** | Pour les GIF, analyser plusieurs frames (pas seulement la premiere). Un GIF peut commencer safe et devenir NSFW | LOW |
| **Queue d'analyse** | Si le rate limiter bloque, mettre l'image en queue au lieu de la supprimer preventive. Analyser des que possible | LOW |
| **Whitelist d'emojis/stickers** | Ignorer les emojis et stickers Discord officiels (ils sont deja moderes). Ne scanner que le contenu custom | LOW |

### Voice Bot

| Feature | Description | Priorite |
|---------|-------------|----------|
| **Statistiques vocales par salon** | Tracker le nombre de participants, la duree moyenne, les pics d'utilisation. Affichable dans le dashboard | MEDIUM |
| **Salons a theme** | Templates de salon pre-configures (Gaming, Musique, Travail). Chaque theme a des permissions, une limite d'utilisateurs et un nom automatique differents | MEDIUM |
| **Invitation par lien** | Generer un lien d'invitation temporaire pour le salon vocal (valide X minutes). L'utilisateur invite est automatiquement autorise meme sans etre dans la whitelist | MEDIUM |
| **Salon "stage"** | Mode presentation : seul le speaker peut parler, les autres sont en ecoute. Le speaker peut donner la parole ponctuellement. Utile pour les events | LOW |
| **Enregistrement vocal** | Option pour enregistrer les conversations vocales (avec consentement de tous les participants). Stockage et telechargement depuis le dashboard | LOW |
| **AFK auto-move** | Deplacer automatiquement les utilisateurs AFK (muet + sourd > X min) vers un canal AFK dedié. Libere les places | LOW |

### Audit Bot

| Feature | Description | Priorite |
|---------|-------------|----------|
| **Detection d'anomalies** | Analyser les patterns d'audit pour detecter les comportements anormaux : moderateur qui ban en masse, role admin ajoute a un compte suspect, channel permissions modifiees en rafale | HIGH |
| **Alertes configurables** | Regles d'alerte per-guild : "Si plus de X bans en Y minutes → notification urgent". "Si un role admin est modifie → notification immediate" | HIGH |
| **Correlation d'evenements** | Lier les evenements connexes : un message supprime → qui l'a supprime (via audit log Discord) → etait-ce automod ou un moderateur ? Timeline unifiee | MEDIUM |
| **Retention et archivage** | Politique de retention configurable : garder les logs X jours puis archiver en cold storage. Compression des anciens logs | MEDIUM |
| **Diff de permissions** | Quand les permissions d'un canal ou role changent, calculer et afficher le diff exact (quels permissions ajoutees/retirees). Plus lisible que les raw values | MEDIUM |
| **Export legaux** | Generer un rapport formate pour Discord Trust & Safety ou les autorites. Inclut : timeline, preuves, contexte, actions prises | LOW |
| **Replay d'evenements** | Interface dans le dashboard pour rejouer les evenements d'une periode donnee. Utile pour comprendre ce qui s'est passe pendant un incident | LOW |
| **Rapport hebdomadaire automatique** | Chaque lundi matin, generer un embed recap dans un canal admin configurable : messages de la semaine, nouveaux membres, departs, infractions (warns/mutes/bans), top 5 actifs, tendances vs semaine precedente (hausse/baisse avec fleches). Zero effort pour les admins. Genere par le analytics-worker et envoye via Redis pub/sub → audit-bot qui poste l'embed | MEDIUM |
| **Auto-archivage des canaux inactifs** | Detecter les canaux texte sans aucun message depuis X jours (configurable, defaut 30j). Les deplacer automatiquement vers une categorie "Archives" en read-only. Notification dans le canal admin avant archivage (48h de delai). Commande `/unarchive <canal>` pour restaurer. Nettoie le serveur automatiquement | MEDIUM |
| **Synthese mensuelle IA** | Chaque 1er du mois, collecter TOUTES les donnees du serveur sur le mois passe (messages, vocal, infractions, membres, tickets, securite, XP, roles, vocal) et envoyer a une IA (Claude API) pour generer une synthese intelligente en langage naturel. L'IA produit : resume executif, points positifs, points d'attention, recommandations concretes, comparaison avec le mois precedent. Le rapport est poste en embed dans le canal admin + archive en BDD + visible dans l'app desktop. Configurable per-guild (activer/desactiver, canal de destination, langue) | HIGH |

### Community Bot (ex Roles Bot)

| Feature | Description | Priorite |
|---------|-------------|----------|
| **Roles temporaires** | Roles avec expiration automatique. Ex: role "VIP" pendant 30 jours, role "Event" pendant la duree d'un evenement. Tache de fond pour le nettoyage | HIGH |
| **Roles exclusifs** | Groupes de roles mutuellement exclusifs. Si un utilisateur prend le role "Equipe Rouge", le role "Equipe Bleue" est retire automatiquement | HIGH |
| **Roles par niveau** | Integration avec le systeme XP : attribuer automatiquement des roles quand l'utilisateur atteint un niveau. Deja possible via level_rewards mais pas gere cote bot | MEDIUM |
| **Limite de roles** | Limiter le nombre de roles qu'un utilisateur peut prendre dans un panel (ex: max 3 couleurs). Configurable per-panel | MEDIUM |
| **Roles conditionnels** | Prerequis pour obtenir un role : avoir un autre role, etre dans le serveur depuis X jours, avoir un niveau minimum. Verification au clic | MEDIUM |
| **Roles par reaction** | Support des reactions (en plus des boutons) pour les anciens panels. Backward compatibility avec les systemes classiques | LOW |
| **Roles saisonniers** | Roles automatiques selon la saison/evenement en cours. Configurable avec des dates de debut/fin | LOW |
| **Roles boost** | Attribution automatique d'un role special aux server boosters. Detection via guild_member_update quand premium_since change | LOW |
| **Systeme de parrainage** | `/parrain @nouveau_membre` : un membre existant parraine un nouveau. Le parrain guide le filleul dans ses premiers jours. Bonus XP pour les deux (+50% XP pour le filleul pendant 7 jours, +25% pour le parrain). Stats de retention : comparer les membres parraines vs non parraines. Gamifie l'accueil et implique la communaute dans l'integration. Limites : 1 parrain par filleul, max 3 filleuls actifs par parrain | HIGH |

---

## Modifications API necessaires pour supporter les nouvelles features

Cette section detaille les entites, tables, endpoints et use cases a ajouter dans l'API backend pour supporter les features listees ci-dessus.

### Phase 1 — Critique

#### 1. Systeme de strikes / escalade progressive (Automod + Moderation)

**Nouvelles tables :**

```sql
CREATE TABLE strike_config (
    guild_id     TEXT NOT NULL,
    window_secs  BIGINT NOT NULL DEFAULT 3600,       -- fenetre glissante (1h)
    thresholds   JSONB NOT NULL DEFAULT '[]',         -- ex: [{"strikes":3,"action":"mute","duration":600},{"strikes":5,"action":"ban"}]
    enabled      BOOLEAN NOT NULL DEFAULT TRUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (guild_id)
);

CREATE TABLE user_strikes (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id     TEXT NOT NULL,
    user_id      TEXT NOT NULL,
    reason       TEXT NOT NULL,
    source       TEXT NOT NULL,                       -- "automod", "moderator", "system"
    infraction_id UUID REFERENCES infractions(id),
    expires_at   TIMESTAMPTZ,                         -- NULL = permanent
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_strikes_guild_user ON user_strikes(guild_id, user_id);
CREATE INDEX idx_strikes_expires ON user_strikes(expires_at) WHERE expires_at IS NOT NULL;
```

**Nouveaux endpoints :**

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/api/strikes/config/{guild_id}` | Config escalade du serveur |
| PUT | `/api/strikes/config/{guild_id}` | Modifier la config escalade |
| GET | `/api/strikes/{guild_id}/{user_id}` | Strikes actifs d'un utilisateur |
| POST | `/api/strikes` | Ajouter un strike (retourne l'action d'escalade si seuil atteint) |
| DELETE | `/api/strikes/{guild_id}/{user_id}` | Reset les strikes d'un utilisateur |

**Nouveau use case : `ManageStrikesUseCase`**

```
add_strike(guild_id, user_id, reason, source) → StrikeResult {
    strikes_count, escalation_action?, escalation_duration?
}
get_active_strikes(guild_id, user_id) → Vec<Strike>
reset_strikes(guild_id, user_id) → ()
get_config(guild_id) → StrikeConfig
save_config(guild_id, config) → StrikeConfig
```

**Integration :** `AnalyzeMessageService` appelle `add_strike()` apres chaque infraction. Si le seuil est atteint, l'action escaladee remplace l'action du scoring.

---

#### 2. Systeme d'appel de sanction (Moderation + Ticket)

**Nouvelles tables :**

```sql
CREATE TABLE appeals (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    user_id         TEXT NOT NULL,
    username        TEXT NOT NULL,
    infraction_id   UUID REFERENCES infractions(id),
    action_id       UUID REFERENCES moderation_actions(id),
    ticket_id       UUID REFERENCES tickets(id),          -- ticket auto-cree
    status          TEXT NOT NULL DEFAULT 'pending',       -- pending, reviewing, accepted, rejected
    reason          TEXT NOT NULL,                         -- justification de l'appel
    reviewer_id     TEXT,
    reviewer_name   TEXT,
    decision_reason TEXT,                                  -- justification de la decision
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at     TIMESTAMPTZ
);
CREATE INDEX idx_appeals_guild_user ON appeals(guild_id, user_id);
CREATE INDEX idx_appeals_status ON appeals(status);
```

**Nouveaux endpoints :**

| Methode | Route | Description |
|---------|-------|-------------|
| POST | `/api/appeals` | Creer un appel (cree aussi un ticket automatiquement) |
| GET | `/api/appeals/{guild_id}` | Lister les appels d'un serveur |
| GET | `/api/appeals/{guild_id}/{user_id}` | Appels d'un utilisateur |
| PATCH | `/api/appeals/{id}/review` | Prendre en charge / decider d'un appel |

---

#### 3. Roles temporaires avec expiration (Roles Bot)

**Nouvelle table :**

```sql
CREATE TABLE temporary_roles (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    role_id     TEXT NOT NULL,
    reason      TEXT,
    assigned_by TEXT NOT NULL,
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_temp_roles_expires ON temporary_roles(expires_at);
CREATE INDEX idx_temp_roles_guild_user ON temporary_roles(guild_id, user_id);
```

**Nouveaux endpoints :**

| Methode | Route | Description |
|---------|-------|-------------|
| POST | `/api/temporary-roles` | Assigner un role temporaire |
| GET | `/api/temporary-roles/{guild_id}` | Lister les roles temporaires actifs |
| GET | `/api/temporary-roles/expiring` | Roles qui expirent bientot (pour le worker) |
| DELETE | `/api/temporary-roles/{id}` | Retirer un role temporaire |

**Nouveau job worker :** `cleanup_temp_roles` dans le moderation-worker (toutes les 60s).

---

#### 4. Actions de moderation programmees (Moderation Bot)

**Nouvelle table :**

```sql
CREATE TABLE scheduled_actions (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id     TEXT NOT NULL,
    target_id    TEXT NOT NULL,
    target_name  TEXT NOT NULL,
    action_type  TEXT NOT NULL,                -- "mute", "ban", "unmute", "unban", "role_add", "role_remove"
    reason       TEXT NOT NULL,
    scheduled_by TEXT NOT NULL,
    execute_at   TIMESTAMPTZ NOT NULL,
    duration     BIGINT,                       -- duree de l'action en secondes (pour mute/ban temp)
    status       TEXT NOT NULL DEFAULT 'pending', -- pending, executed, cancelled
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_scheduled_execute ON scheduled_actions(execute_at) WHERE status = 'pending';
```

**Nouveaux endpoints :**

| Methode | Route | Description |
|---------|-------|-------------|
| POST | `/api/scheduled-actions` | Programmer une action |
| GET | `/api/scheduled-actions/{guild_id}` | Actions programmees d'un serveur |
| DELETE | `/api/scheduled-actions/{id}` | Annuler une action programmee |
| GET | `/api/scheduled-actions/pending` | Actions a executer (pour le worker) |

**Nouveau job worker :** `execute_scheduled_actions` dans le moderation-worker (toutes les 30s).

---

#### 5. Notes sur les utilisateurs (Moderation Bot)

**Nouvelle table :**

```sql
CREATE TABLE user_notes (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id     TEXT NOT NULL,
    user_id      TEXT NOT NULL,
    author_id    TEXT NOT NULL,
    author_name  TEXT NOT NULL,
    content      TEXT NOT NULL,
    category     TEXT DEFAULT 'general',       -- "general", "warning", "positive", "context"
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_notes_guild_user ON user_notes(guild_id, user_id);
```

**Nouveaux endpoints :**

| Methode | Route | Description |
|---------|-------|-------------|
| POST | `/api/notes` | Ajouter une note |
| GET | `/api/notes/{guild_id}/{user_id}` | Notes d'un utilisateur |
| DELETE | `/api/notes/{id}` | Supprimer une note |

**Integration :** Ajouter `notes: Vec<UserNote>` dans `UserDossier` (watched users).

---

#### 6. Convocations moderateur `/call` (Moderation Bot)

**Nouvelle table :**

```sql
CREATE TABLE moderation_calls (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    channel_id      TEXT NOT NULL,          -- salon textuel cree
    moderator_id    TEXT NOT NULL,
    moderator_name  TEXT NOT NULL,
    target_id       TEXT NOT NULL,
    target_name     TEXT NOT NULL,
    reason          TEXT,
    status          TEXT NOT NULL DEFAULT 'open',  -- open, closed
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_at       TIMESTAMPTZ
);
CREATE INDEX idx_calls_guild ON moderation_calls(guild_id, status);
CREATE INDEX idx_calls_target ON moderation_calls(guild_id, target_id);
```

**Nouveaux endpoints :**

| Methode | Route | Description |
|---------|-------|-------------|
| POST | `/api/moderation/calls` | Creer une convocation (log) |
| GET | `/api/moderation/calls/{guild_id}` | Lister les convocations d'un serveur |
| GET | `/api/moderation/calls/{guild_id}/{user_id}` | Convocations d'un utilisateur |
| PATCH | `/api/moderation/calls/{id}/close` | Fermer une convocation |

**Logique metier du bot :**

```
/call <user> [raison]
  |
  v
1. Creer un salon textuel "call-{username}" dans une categorie dediee
   - Permissions : moderateur + cible + admins seulement
   - @everyone deny view_channel
  |
2. Envoyer un embed d'accueil dans le salon :
   - Card info : qui convoque, qui est convoque, raison
   - Bouton [❌ Fermer la convocation]
  |
3. Mentionner la cible dans le salon (<@user>)
  |
4. POST /api/moderation/calls (log en BDD)
  |
5. A la fermeture (bouton ou /call close) :
   - PATCH /api/moderation/calls/{id}/close
   - Supprimer le salon Discord
   - Log dans l'historique de moderation
```

**Card embed d'accueil :**
```
┌─ BLURPLE (0x5865F2) ────────────────────────────┐
│ 📞 Convocation                           [avatar]│
│                                                 │
│ 👮 Moderateur     👤 Convoque                    │
│ <@moderator>      <@target>                      │
│                                                 │
│ 📝 Raison                                       │
│ Discussion concernant votre comportement recen  │
│                                                 │
│ Ce salon est prive. Seuls vous et l'equipe de   │
│ moderation pouvez le voir.                      │
│                                                 │
│ [❌ Fermer la convocation]                       │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Moderation • 30/03/2026 14h35          │
└─────────────────────────────────────────────────┘
```

**Integration :** Les convocations apparaissent dans `/history <user>` et dans le dossier `UserDossier` de l'app desktop.

---

### Phase 2 — Haute priorite

#### 6. Suivi SLA des tickets (Ticket Bot)

**Modifications table `tickets` :**

```sql
ALTER TABLE tickets ADD COLUMN first_response_at TIMESTAMPTZ;
ALTER TABLE tickets ADD COLUMN sla_target_minutes INT;
ALTER TABLE tickets ADD COLUMN sla_breached BOOLEAN DEFAULT FALSE;
```

**Nouvelle table :**

```sql
CREATE TABLE sla_config (
    guild_id            TEXT PRIMARY KEY,
    target_minutes_low  INT NOT NULL DEFAULT 1440,   -- 24h
    target_minutes_medium INT NOT NULL DEFAULT 480,  -- 8h
    target_minutes_high INT NOT NULL DEFAULT 120,    -- 2h
    target_minutes_urgent INT NOT NULL DEFAULT 30,   -- 30min
    alert_channel_id    TEXT,
    enabled             BOOLEAN NOT NULL DEFAULT FALSE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Nouveaux endpoints :**

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/api/sla/config/{guild_id}` | Config SLA |
| PUT | `/api/sla/config/{guild_id}` | Modifier la config SLA |
| GET | `/api/sla/metrics/{guild_id}` | Metriques SLA (temps moyen reponse, % dans les temps) |
| GET | `/api/sla/breaches/{guild_id}` | Tickets en breach SLA |

**Modification use case :** `ManageTicketsService::reply_ticket()` met a jour `first_response_at` si c'est la premiere reponse staff.

**Nouveau job worker :** `check_sla_breaches` dans le analytics-worker (toutes les 5 min).

---

#### 7. Satisfaction post-fermeture (Ticket Bot)

**Nouvelle table :**

```sql
CREATE TABLE ticket_satisfaction (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ticket_id   UUID NOT NULL REFERENCES tickets(id),
    guild_id    TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    rating      SMALLINT NOT NULL CHECK (rating BETWEEN 1 AND 5),
    comment     TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE UNIQUE INDEX idx_satisfaction_ticket ON ticket_satisfaction(ticket_id);
```

**Nouveaux endpoints :**

| Methode | Route | Description |
|---------|-------|-------------|
| POST | `/api/satisfaction` | Soumettre un avis |
| GET | `/api/satisfaction/{guild_id}` | Stats satisfaction (moyenne, distribution) |
| GET | `/api/satisfaction/ticket/{ticket_id}` | Avis d'un ticket |

---

#### 8. Streaks et multiplicateurs XP (Stats Bot)

**Modification table `user_levels` :**

```sql
ALTER TABLE user_levels ADD COLUMN streak_days INT NOT NULL DEFAULT 0;
ALTER TABLE user_levels ADD COLUMN last_activity_date DATE;
ALTER TABLE user_levels ADD COLUMN longest_streak INT NOT NULL DEFAULT 0;
```

**Modification use case :** `ManageLevelsService::add_xp()` :
1. Verifie si `last_activity_date` = aujourd'hui → pas de changement de streak
2. Si `last_activity_date` = hier → `streak_days += 1`
3. Sinon → `streak_days = 1` (reset)
4. Multiplicateur : x1.0 (0-2j), x1.5 (3-6j), x2.0 (7-29j), x3.0 (30j+)
5. XP final = XP base * multiplicateur

**Nouveau endpoint :**

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/api/levels/{guild_id}/streaks` | Top streaks du serveur |

---

#### 9. Cooldown XP + boosts par canal/role (Stats Bot)

**Modification table `levels_config` :**

```sql
-- xp_cooldown_secs existe deja
ALTER TABLE levels_config ADD COLUMN channel_multipliers JSONB DEFAULT '{}';  -- {"channel_id": 2.0}
ALTER TABLE levels_config ADD COLUMN role_multipliers JSONB DEFAULT '{}';     -- {"role_id": 1.5}
```

**Modification use case :** `ManageLevelsService::add_xp()` :
1. Verifie `last_xp_at` : si < cooldown → ignore
2. Cherche le multiplicateur canal dans `channel_multipliers`
3. Cherche le meilleur multiplicateur role dans `role_multipliers`
4. XP final = XP base * max(channel_mult, 1.0) * max(role_mult, 1.0) * streak_mult

---

#### 10. Cache de hash perceptuel d'images (Image Bot)

**Nouvelle table :**

```sql
CREATE TABLE image_hash_cache (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id   TEXT NOT NULL,
    phash      TEXT NOT NULL,                 -- hash perceptuel (hex string)
    action     TEXT NOT NULL,                 -- action prise (warn/delete/mute/ban)
    label      TEXT NOT NULL,                 -- "nsfw" ou "illicit"
    confidence REAL NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_hash_guild ON image_hash_cache(guild_id, phash);
```

**Nouveaux endpoints :**

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/api/image-hashes/{guild_id}/{phash}` | Verifier si un hash existe |
| POST | `/api/image-hashes` | Enregistrer un hash (apres analyse) |

**Integration :** `AnalyzeImageService` :
1. Avant l'inference ONNX, calculer le pHash de l'image
2. Verifier dans le cache : si match → retourner l'action directement (pas d'inference)
3. Apres l'inference, si flag detecte → stocker le hash dans le cache

---

#### 11. Detection d'anomalies (Audit Bot)

**Nouvelle table :**

```sql
CREATE TABLE anomaly_rules (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    rule_type   TEXT NOT NULL,             -- "mass_ban", "role_escalation", "permission_change", "channel_delete_burst"
    threshold   INT NOT NULL,              -- ex: 5 bans en 10 min
    window_secs INT NOT NULL DEFAULT 600,
    severity    TEXT NOT NULL DEFAULT 'high',
    enabled     BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE anomaly_events (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    rule_id     UUID REFERENCES anomaly_rules(id),
    rule_type   TEXT NOT NULL,
    actor_id    TEXT NOT NULL,
    actor_name  TEXT NOT NULL,
    details     JSONB NOT NULL DEFAULT '{}',
    severity    TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_anomaly_guild ON anomaly_events(guild_id, created_at DESC);
```

**Nouveaux endpoints :**

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/api/anomalies/rules/{guild_id}` | Regles d'anomalie configurees |
| PUT | `/api/anomalies/rules/{guild_id}` | Configurer les regles |
| GET | `/api/anomalies/events/{guild_id}` | Evenements d'anomalie detectes |
| POST | `/api/anomalies/check` | Verifier si un pattern est anormal (appele par l'audit bot) |

**Integration :** L'Audit Bot appelle `POST /api/anomalies/check` avec le type d'evenement et l'acteur. L'API verifie contre les regles configurees et declenche une alerte si le seuil est depasse.

---

#### 12. Synthese mensuelle IA (Audit Bot + Analytics Worker + Claude API)

**Nouvelles tables :**

```sql
CREATE TABLE monthly_reports (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    month           DATE NOT NULL,                      -- 1er du mois (ex: 2026-03-01)
    raw_data        JSONB NOT NULL,                     -- donnees brutes collectees
    ai_summary      TEXT NOT NULL,                      -- synthese generee par l'IA
    ai_highlights   JSONB NOT NULL DEFAULT '[]',        -- points cles extraits
    ai_recommendations JSONB NOT NULL DEFAULT '[]',     -- recommandations
    comparison      JSONB,                              -- comparaison mois precedent
    tokens_used     INT NOT NULL DEFAULT 0,             -- tokens IA consommes
    generated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    posted_at       TIMESTAMPTZ,                        -- quand le rapport a ete poste sur Discord
    UNIQUE(guild_id, month)
);
CREATE INDEX idx_monthly_reports_guild ON monthly_reports(guild_id, month DESC);

CREATE TABLE monthly_report_config (
    guild_id        TEXT PRIMARY KEY,
    enabled         BOOLEAN NOT NULL DEFAULT FALSE,
    channel_id      TEXT,                               -- canal admin ou poster
    language        TEXT NOT NULL DEFAULT 'fr',          -- langue de la synthese
    ai_provider     TEXT NOT NULL DEFAULT 'claude',      -- claude, openai
    include_recommendations BOOLEAN NOT NULL DEFAULT TRUE,
    include_comparison BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Nouveaux endpoints :**

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/api/monthly-reports/{guild_id}` | Lister les rapports mensuels |
| GET | `/api/monthly-reports/{guild_id}/{month}` | Detail d'un rapport |
| POST | `/api/monthly-reports/{guild_id}/generate` | Generer manuellement un rapport |
| GET | `/api/monthly-reports/config/{guild_id}` | Config du rapport mensuel |
| PUT | `/api/monthly-reports/config/{guild_id}` | Modifier la config |

**Pipeline de generation :**

```
1er du mois a 8h00 (job analytics-worker)
  |
  v
1. COLLECTE DES DONNEES (requetes SQL agregees)
   |
   +-- Messages : total, par canal, par jour, pics d'activite
   +-- Vocal : heures totales, duree moyenne session, pics
   +-- Membres : arrivees, departs, taux retention, comptes suspects
   +-- Infractions : total par type (warn/delete/mute/ban), recidivistes, evolution
   +-- Tickets : ouverts, fermes, temps moyen reponse, satisfaction moyenne
   +-- Securite : raids detectes, comptes suspects, quarantaines
   +-- XP/Niveaux : nouveaux niveaux atteints, top progressions
   +-- Moderation : actions par moderateur, tendances
   +-- Roles : changements, panels les plus utilises
   |
2. COMPARAISON AVEC LE MOIS PRECEDENT
   |
   +-- Delta pour chaque metrique (hausse/baisse en %)
   +-- Tendances sur 3 mois si disponible
   |
3. CONSTRUCTION DU PROMPT IA
   |
   +-- Contexte : "Tu es un analyste de communaute Discord."
   +-- Donnees : JSON structure avec toutes les metriques
   +-- Instructions : generer resume, points positifs, alertes, recommandations
   +-- Langue : configurable (fr/en)
   |
4. APPEL CLAUDE API (ou OpenAI selon config)
   |
   +-- Model : claude-sonnet-4-6 (bon rapport qualite/prix pour les syntheses)
   +-- Max tokens : 2000
   +-- Temperature : 0.3 (factuel, pas creatif)
   |
5. PARSING DE LA REPONSE
   |
   +-- Extraction : resume, highlights[], recommendations[]
   +-- Stockage en BDD (monthly_reports)
   |
6. PUBLICATION
   |
   +-- POST vers Redis pub/sub → Audit Bot poste l'embed dans le canal admin
   +-- Notification WebSocket → Desktop App affiche le nouveau rapport
```

**Exemple de prompt envoye a l'IA :**

```
Tu es un analyste de communaute Discord. Voici les donnees du serveur "MonServeur"
pour le mois de mars 2026. Genere une synthese en francais.

Donnees :
- Messages : 45,230 (fevrier : 38,100, +18.7%)
- Temps vocal : 1,234 heures (fevrier : 980h, +25.9%)
- Nouveaux membres : 156 (fevrier : 132)
- Departs : 42 (fevrier : 38)
- Infractions : 89 warns, 12 mutes, 3 bans (fevrier : 95 warns, 15 mutes, 5 bans)
- Tickets : 23 ouverts, 21 fermes, temps moyen reponse 45min
- Raids : 1 detecte (12 comptes, neutralise en 30s)
- Top canal : #general (12,340 msgs), #gaming (8,210 msgs)
- Pic d'activite : samedi 21h (moyenne 89 msgs/h)

Reponds en JSON :
{
  "summary": "Resume executif en 3-5 phrases",
  "highlights": ["point positif 1", "point positif 2"],
  "concerns": ["point d'attention 1"],
  "recommendations": ["recommandation 1", "recommandation 2"],
  "health_score": 85
}
```

**Exemple de reponse IA :**

```json
{
  "summary": "Mars a ete un mois dynamique pour MonServeur avec une hausse significative de l'activite (+18.7% messages, +25.9% vocal). La communaute grandit avec 156 nouveaux membres et un taux de retention de 73%. Les infractions sont en baisse (-6.3% warns, -20% mutes, -40% bans), signe d'une communaute qui se stabilise.",
  "highlights": [
    "Croissance soutenue : +18.7% de messages et +25.9% de temps vocal",
    "Infractions en nette baisse : -40% de bans par rapport a fevrier",
    "Raid neutralise en 30 secondes, systeme de securite efficace",
    "Temps de reponse tickets excellent (45 min en moyenne)"
  ],
  "concerns": [
    "42 departs ce mois (taux de depart 27%) — surveiller les raisons",
    "Pic d'activite concentre le samedi soir — risque de sous-moderation"
  ],
  "recommendations": [
    "Renforcer la moderation le samedi soir 20h-23h (pic d'activite + infractions)",
    "Analyser les raisons des 42 departs — sondage optionnel aux membres partants",
    "Feliciter les 5 membres les plus actifs pour encourager l'engagement"
  ],
  "health_score": 85
}
```

**Card embed postee par l'Audit Bot :**

```
┌─ BLURPLE (0x5865F2) ────────────────────────────────────┐
│ 📊 Synthese mensuelle — Mars 2026              🏥 85/100│
│                                                         │
│ Mars a ete un mois dynamique pour MonServeur avec une   │
│ hausse significative de l'activite (+18.7% messages,    │
│ +25.9% vocal). La communaute grandit avec 156 nouveaux  │
│ membres et un taux de retention de 73%.                 │
│                                                         │
│ ✅ Points positifs                                       │
│ • Croissance soutenue : +18.7% messages, +25.9% vocal  │
│ • Infractions en nette baisse : -40% de bans            │
│ • Raid neutralise en 30s                                │
│ • Temps reponse tickets : 45 min                        │
│                                                         │
│ ⚠️ Points d'attention                                    │
│ • 42 departs (taux 27%) — surveiller                    │
│ • Pic samedi soir — risque sous-moderation              │
│                                                         │
│ 💡 Recommandations                                       │
│ • Renforcer moderation samedi 20h-23h                   │
│ • Analyser les raisons des departs                      │
│ • Feliciter les top 5 membres actifs                    │
│                                                         │
│ 📈 vs Fevrier : msgs +18.7% · vocal +25.9% · bans -40% │
│                                                         │
│ ─────────────────────────────────────────────────────── │
│ Sentinel Analytics • Genere le 01/04/2026 a 08h00       │
└─────────────────────────────────────────────────────────┘
```

**Configuration requise (env) :**

```env
# IA Provider pour les syntheses mensuelles
ANTHROPIC_API_KEY=sk-ant-...          # Pour Claude API
# ou
OPENAI_API_KEY=sk-...                  # Pour OpenAI (alternative)
```

**Nouveau job worker :** `generate_monthly_report` dans le analytics-worker (cron : 1er du mois a 8h00).

---

### Phase 3 — Moyenne priorite

#### 12. Roles exclusifs et conditionnels (Roles Bot)

**Modifications table `role_panels` :**

```sql
ALTER TABLE role_panels ADD COLUMN exclusive_group TEXT;       -- groupe d'exclusivite (NULL = pas exclusif)
ALTER TABLE role_panels ADD COLUMN required_roles JSONB DEFAULT '[]';  -- roles prerequis
ALTER TABLE role_panels ADD COLUMN min_server_days INT;               -- jours minimum dans le serveur
ALTER TABLE role_panels ADD COLUMN min_level INT;                     -- niveau minimum requis
```

**Modification use case :** Ajouter une verification dans le handler du roles-bot :
1. Verifier les prerequis (`required_roles`, `min_server_days`, `min_level`)
2. Si `exclusive_group` est defini, retirer les autres roles du meme groupe avant d'ajouter

---

#### 13. Templates de reponse tickets (Ticket Bot)

**Nouvelle table :**

```sql
CREATE TABLE ticket_templates (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    name        TEXT NOT NULL,
    content     TEXT NOT NULL,
    category    TEXT DEFAULT 'general',
    usage_count INT NOT NULL DEFAULT 0,
    created_by  TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_templates_guild ON ticket_templates(guild_id);
```

**Nouveaux endpoints :**

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/api/ticket-templates/{guild_id}` | Lister les templates |
| POST | `/api/ticket-templates` | Creer un template |
| DELETE | `/api/ticket-templates/{id}` | Supprimer un template |
| POST | `/api/ticket-templates/{id}/use` | Incrementer le compteur d'usage |

---

#### 14. Historique des pseudos (Audit Bot)

**Nouvelle table :**

```sql
CREATE TABLE username_history (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    old_name    TEXT NOT NULL,
    new_name    TEXT NOT NULL,
    change_type TEXT NOT NULL DEFAULT 'nickname', -- "nickname", "username", "display_name"
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_name_history_user ON username_history(guild_id, user_id, created_at DESC);
```

**Nouveau endpoint :**

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/api/name-history/{guild_id}/{user_id}` | Historique des noms |
| POST | `/api/name-history` | Enregistrer un changement de nom |

**Integration :** L'audit bot appelle `POST /api/name-history` sur chaque `guild_member_update` qui inclut un changement de pseudo.

---

#### 15. Statistiques vocales par salon (Voice Bot)

**Nouvelle table :**

```sql
CREATE TABLE voice_channel_stats (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    channel_id      TEXT NOT NULL,
    owner_id        TEXT NOT NULL,
    peak_members    INT NOT NULL DEFAULT 0,
    total_duration  BIGINT NOT NULL DEFAULT 0,       -- duree totale en secondes
    total_joins     INT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_at       TIMESTAMPTZ
);
CREATE INDEX idx_voice_stats_guild ON voice_channel_stats(guild_id, created_at DESC);
```

**Nouveau endpoint :**

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/api/voice-stats/{guild_id}` | Stats vocales agregees (salons crees, duree moyenne, pic) |
| POST | `/api/voice-stats` | Enregistrer les stats a la fermeture d'un salon |

---

### Phase 4 — Nice-to-have

#### 16. Templates d'avertissement (Moderation Bot)

**Nouvelle table :**

```sql
CREATE TABLE warning_templates (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    name        TEXT NOT NULL,
    reason      TEXT NOT NULL,
    gravity     TEXT NOT NULL DEFAULT 'medium',
    category    TEXT DEFAULT 'general',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Endpoint :** CRUD standard `/api/warning-templates/{guild_id}`

---

#### 17. Achievements / Badges (Stats Bot)

**Nouvelles tables :**

```sql
CREATE TABLE badge_definitions (
    id          TEXT PRIMARY KEY,             -- "first_message", "100h_vocal", "level_50"
    name        TEXT NOT NULL,
    description TEXT NOT NULL,
    emoji       TEXT NOT NULL,
    category    TEXT NOT NULL,                -- "messages", "vocal", "levels", "moderation", "special"
    condition   JSONB NOT NULL               -- {"type": "messages", "threshold": 1000}
);

CREATE TABLE user_badges (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    badge_id    TEXT NOT NULL REFERENCES badge_definitions(id),
    awarded_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(guild_id, user_id, badge_id)
);
CREATE INDEX idx_badges_user ON user_badges(guild_id, user_id);
```

**Endpoints :**

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/api/badges` | Lister tous les badges disponibles |
| GET | `/api/badges/{guild_id}/{user_id}` | Badges d'un utilisateur |
| POST | `/api/badges/check` | Verifier et attribuer les badges gagnes |

**Integration :** Appeler `POST /api/badges/check` apres chaque gain d'XP, message, action vocale. L'API verifie les conditions et attribue les nouveaux badges.

---

#### 18. Reaction roles (Roles Bot)

**Nouvelle table :**

```sql
CREATE TABLE reaction_roles (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    channel_id  TEXT NOT NULL,
    message_id  TEXT NOT NULL,
    emoji       TEXT NOT NULL,
    role_id     TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(message_id, emoji)
);
```

**Endpoints :** CRUD standard `/api/reaction-roles/{guild_id}`

---

### Resume des modifications API

| Phase | Nouvelles tables | Nouveaux endpoints | Tables modifiees | Nouveaux jobs worker |
|-------|-----------------|-------------------|-----------------|---------------------|
| 1 | 5 (strikes, appeals, temp_roles, scheduled_actions, notes) | 18 | 0 | 2 (cleanup_temp_roles, execute_scheduled) |
| 2 | 4 (sla_config, satisfaction, hash_cache, anomalies x2) | 14 | 3 (tickets, user_levels, levels_config) | 1 (check_sla) |
| 3 | 4 (templates, name_history, voice_stats, role_panels alter) | 9 | 1 (role_panels) | 0 |
| 4 | 4 (warning_templates, badges x2, reaction_roles) | 8 | 0 | 0 |
| **Total** | **17 tables** | **49 endpoints** | **4 alterations** | **3 jobs** |

---

## Modifications Application Desktop necessaires

Cette section detaille les pages, composables, types TypeScript, commandes Tauri et traits Rust a ajouter dans l'application desktop pour supporter les nouvelles features.

### Etat actuel de l'application

| Categorie | Nombre actuel |
|-----------|--------------|
| Routes/Pages | 26 |
| Composables | 28 |
| Commandes Tauri | 53 (43 queries + 10 mutations) |
| Types TypeScript | 50+ |
| Traits Repository (Rust) | 15 |

---

### Phase 1 — Critique

#### Nouvelles pages

**`StrikesPage.vue`** — Gestion du systeme de strikes/escalade

- Config per-guild : fenetre glissante, seuils d'escalade (3 strikes = mute, 5 = ban, etc.)
- Liste des strikes actifs par utilisateur avec filtre/recherche
- Actions : reset les strikes d'un utilisateur
- Graphique : evolution des strikes dans le temps
- Integration : lien vers l'infraction source de chaque strike

**`AppealsPage.vue`** — Gestion des appels de sanction

- Liste des appels avec filtres par statut (pending/reviewing/accepted/rejected)
- Detail d'un appel : infraction source, raison de l'appel, historique du membre
- Actions : prendre en charge, accepter, rejeter avec justification
- Lien vers le ticket auto-cree pour l'appel
- Compteur d'appels en attente dans la sidebar

**`ScheduledActionsPage.vue`** — Actions de moderation programmees

- Liste des actions programmees avec compte a rebours
- Formulaire : cible, type d'action, date/heure d'execution, raison
- Actions : annuler une action programmee
- Indicateur visuel : a venir dans <1h (rouge), <24h (orange), >24h (vert)

#### Pages existantes a enrichir

**`WatchedUsersPage.vue`** — Ajouter onglet Notes

- Nouveau tab "Notes" dans le dossier utilisateur
- Liste des notes avec categorie (general/warning/positive/context)
- Formulaire d'ajout de note (auteur auto-rempli)
- Suppression de note

**`ModerationPage.vue`** — Ajouter lien vers strikes + convocations

- Afficher le nombre de strikes actifs dans l'historique utilisateur
- Bouton "Voir les strikes" qui redirige vers StrikesPage filtree
- Section "Convocations actives" : liste des appels en cours avec statut

**`CallsPage.vue`** — Gestion des convocations moderateur

- Liste des convocations actives et passees avec filtres (statut, moderateur, cible)
- Detail d'une convocation : moderateur, cible, raison, date, duree, statut
- Lien vers l'historique de moderation de la cible
- Actions : fermer une convocation depuis le desktop
- Recherche par nom d'utilisateur

#### Nouveaux composables

| Composable | Commandes Tauri utilisees | Description |
|------------|--------------------------|-------------|
| `useStrikes` | `get_strikes_config`, `set_strikes_config`, `get_user_strikes`, `reset_user_strikes` | Config escalade + strikes par utilisateur |
| `useAppeals` | `get_appeals`, `review_appeal` | Liste, filtre et gestion des appels |
| `useScheduledActions` | `get_scheduled_actions`, `create_scheduled_action`, `cancel_scheduled_action` | Actions programmees CRUD |
| `useUserNotes` | `get_user_notes`, `add_user_note`, `delete_user_note` | Notes sur les utilisateurs |
| `useCalls` | `get_calls`, `get_user_calls`, `close_call` | Convocations moderateur |

#### Nouveaux types TypeScript

```typescript
interface Strike {
  id: string;
  guild_id: string;
  user_id: string;
  reason: string;
  source: "automod" | "moderator" | "system";
  infraction_id?: string;
  expires_at?: string;
  created_at: string;
}

interface StrikeConfig {
  guild_id: string;
  window_secs: number;
  thresholds: { strikes: number; action: string; duration?: number }[];
  enabled: boolean;
}

interface Appeal {
  id: string;
  guild_id: string;
  user_id: string;
  username: string;
  infraction_id?: string;
  ticket_id?: string;
  status: "pending" | "reviewing" | "accepted" | "rejected";
  reason: string;
  reviewer_id?: string;
  reviewer_name?: string;
  decision_reason?: string;
  created_at: string;
  resolved_at?: string;
}

interface ScheduledAction {
  id: string;
  guild_id: string;
  target_id: string;
  target_name: string;
  action_type: "mute" | "ban" | "unmute" | "unban" | "role_add" | "role_remove";
  reason: string;
  scheduled_by: string;
  execute_at: string;
  duration?: number;
  status: "pending" | "executed" | "cancelled";
  created_at: string;
}

interface UserNote {
  id: string;
  guild_id: string;
  user_id: string;
  author_id: string;
  author_name: string;
  content: string;
  category: "general" | "warning" | "positive" | "context";
  created_at: string;
}

// Convocations moderateur
interface ModerationCall {
  id: string;
  guild_id: string;
  channel_id: string;
  moderator_id: string;
  moderator_name: string;
  target_id: string;
  target_name: string;
  reason?: string;
  status: "open" | "closed";
  created_at: string;
  closed_at?: string;
}
```

#### Nouvelles commandes Tauri (16)

```
get_strikes_config(guild_id) → StrikeConfig
set_strikes_config(guild_id, config) → StrikeConfig
get_user_strikes(guild_id, user_id) → Vec<Strike>
reset_user_strikes(guild_id, user_id) → ()
get_appeals(guild_id) → Vec<Appeal>
get_user_appeals(guild_id, user_id) → Vec<Appeal>
review_appeal(appeal_id, status, decision_reason) → Appeal
get_user_notes(guild_id, user_id) → Vec<UserNote>
add_user_note(guild_id, user_id, content, category) → UserNote
delete_user_note(note_id) → ()
get_scheduled_actions(guild_id) → Vec<ScheduledAction>
create_scheduled_action(...) → ScheduledAction
cancel_scheduled_action(action_id) → ()
get_calls(guild_id) → Vec<ModerationCall>
get_user_calls(guild_id, user_id) → Vec<ModerationCall>
close_call(call_id) → ()
```

#### Nouveaux traits Rust (Tauri backend)

```rust
pub trait StrikesRepository {
    fn get_config(&self, guild_id: String) -> BoxFut<StrikeConfig>;
    fn set_config(&self, guild_id: String, config: StrikeConfig) -> BoxFut<StrikeConfig>;
    fn get_user_strikes(&self, guild_id: String, user_id: String) -> BoxFut<Vec<Strike>>;
    fn reset_strikes(&self, guild_id: String, user_id: String) -> BoxFut<()>;
}

pub trait AppealsRepository {
    fn get_appeals(&self, guild_id: String) -> BoxFut<Vec<Appeal>>;
    fn review_appeal(&self, id: String, status: String, reason: String) -> BoxFut<Appeal>;
}

pub trait UserNotesRepository {
    fn get_notes(&self, guild_id: String, user_id: String) -> BoxFut<Vec<UserNote>>;
    fn add_note(&self, guild_id: String, user_id: String, content: String, category: String) -> BoxFut<UserNote>;
    fn delete_note(&self, note_id: String) -> BoxFut<()>;
}

pub trait ScheduledActionsRepository {
    fn get_actions(&self, guild_id: String) -> BoxFut<Vec<ScheduledAction>>;
    fn create_action(&self, action: ScheduledAction) -> BoxFut<ScheduledAction>;
    fn cancel_action(&self, action_id: String) -> BoxFut<()>;
}

pub trait CallsRepository {
    fn get_calls(&self, guild_id: String) -> BoxFut<Vec<ModerationCall>>;
    fn get_user_calls(&self, guild_id: String, user_id: String) -> BoxFut<Vec<ModerationCall>>;
    fn close_call(&self, call_id: String) -> BoxFut<()>;
}
```

---

### Phase 2 — Haute priorite

#### Nouvelles pages

**`AnomaliesPage.vue`** — Detection d'anomalies

- Configuration des regles d'anomalie per-guild (type, seuil, fenetre, severite)
- Timeline des evenements d'anomalie detectes
- Detail : qui a fait quoi, combien de fois, dans quelle fenetre
- Badges de severite avec code couleur
- Notifications temps reel via WebSocket

**`TemporaryRolesPage.vue`** — Roles temporaires

- Liste des roles temporaires actifs avec compte a rebours
- Formulaire d'assignation : utilisateur, role, duree, raison
- Actions : retirer un role temporaire
- Filtre par role, utilisateur, date d'expiration

**`MonthlyReportsPage.vue`** — Syntheses mensuelles IA

- Liste des rapports mensuels generes avec date, health score, statut
- Detail d'un rapport : resume IA complet, highlights, concerns, recommandations
- Graphique health score sur 12 mois (evolution sante du serveur)
- Config per-guild : activer/desactiver, canal, langue, provider IA
- Bouton "Generer maintenant" pour forcer un rapport hors planning
- Comparaison cote a cote de 2 mois
- Export PDF du rapport

#### Pages existantes a enrichir

**`TicketsPage.vue`** — Ajouter SLA + Satisfaction

- Indicateur SLA sur chaque ticket (dans les temps / en retard)
- Onglet "SLA" : config des temps cibles par priorite
- Onglet "Satisfaction" : note moyenne, distribution des notes, commentaires
- Badge SLA dans la liste des tickets (vert/orange/rouge)

**`LevelsPage.vue`** — Ajouter Streaks + Boosts

- Onglet "Streaks" : classement des streaks actifs, plus longues streaks
- Config : multiplicateurs par canal et par role (sliders)
- Affichage du multiplicateur actuel a cote du niveau de chaque utilisateur

#### Nouveaux composables

| Composable | Description |
|------------|-------------|
| `useAnomalies` | Regles d'anomalie + evenements detectes |
| `useTemporaryRoles` | Roles temporaires CRUD |
| `useSLA` | Config SLA + metriques + breaches |
| `useTicketSatisfaction` | Stats satisfaction + avis par ticket |
| `useXpStreaks` | Streaks XP, classement, multiplicateurs |
| `useMonthlyReports` | Rapports mensuels IA, config, generation manuelle |

#### Nouveaux types TypeScript

```typescript
interface AnomalyRule {
  id: string;
  guild_id: string;
  rule_type: "mass_ban" | "role_escalation" | "permission_change" | "channel_delete_burst";
  threshold: number;
  window_secs: number;
  severity: string;
  enabled: boolean;
}

interface AnomalyEvent {
  id: string;
  guild_id: string;
  rule_type: string;
  actor_id: string;
  actor_name: string;
  details: Record<string, unknown>;
  severity: string;
  created_at: string;
}

interface TemporaryRole {
  id: string;
  guild_id: string;
  user_id: string;
  role_id: string;
  reason?: string;
  assigned_by: string;
  expires_at: string;
  created_at: string;
}

interface SLAConfig {
  guild_id: string;
  target_minutes_low: number;
  target_minutes_medium: number;
  target_minutes_high: number;
  target_minutes_urgent: number;
  alert_channel_id?: string;
  enabled: boolean;
}

interface SLAMetrics {
  avg_response_time: number;
  avg_resolution_time: number;
  on_time_percentage: number;
  breached_count: number;
}

interface TicketSatisfaction {
  ticket_id: string;
  rating: number;
  comment?: string;
  created_at: string;
}

interface SatisfactionStats {
  avg_rating: number;
  total_responses: number;
  distribution: Record<number, number>;
}

interface UserStreak {
  user_id: string;
  username: string;
  streak_days: number;
  longest_streak: number;
  last_activity_date: string;
  xp_multiplier: number;
}

// Synthese mensuelle IA
interface MonthlyReport {
  id: string;
  guild_id: string;
  month: string;                           // "2026-03-01"
  ai_summary: string;                     // Resume en langage naturel
  ai_highlights: string[];                // Points positifs
  ai_concerns: string[];                  // Points d'attention
  ai_recommendations: string[];           // Recommandations
  health_score: number;                   // 0-100
  comparison?: {                          // vs mois precedent
    messages_delta: number;               // % change
    vocal_delta: number;
    members_delta: number;
    infractions_delta: number;
  };
  tokens_used: number;
  generated_at: string;
  posted_at?: string;
}

interface MonthlyReportConfig {
  guild_id: string;
  enabled: boolean;
  channel_id?: string;
  language: "fr" | "en";
  ai_provider: "claude" | "openai";
  include_recommendations: boolean;
  include_comparison: boolean;
}
```

#### Nouvelles commandes Tauri (19)

```
get_anomaly_rules(guild_id) → Vec<AnomalyRule>
update_anomaly_rules(guild_id, rules) → Vec<AnomalyRule>
get_anomaly_events(guild_id) → Vec<AnomalyEvent>
get_temporary_roles(guild_id) → Vec<TemporaryRole>
assign_temporary_role(guild_id, user_id, role_id, duration_secs) → TemporaryRole
remove_temporary_role(id) → ()
get_sla_config(guild_id) → SLAConfig
set_sla_config(guild_id, config) → SLAConfig
get_sla_metrics(guild_id) → SLAMetrics
get_sla_breaches(guild_id) → Vec<Ticket>
get_satisfaction_stats(guild_id) → SatisfactionStats
get_ticket_satisfaction(ticket_id) → Option<TicketSatisfaction>
get_user_streak(guild_id, user_id) → UserStreak
get_top_streaks(guild_id, limit) → Vec<UserStreak>
get_streak_leaderboard(guild_id) → Vec<UserStreak>
get_monthly_reports(guild_id) → Vec<MonthlyReport>
get_monthly_report(guild_id, month) → MonthlyReport
generate_monthly_report(guild_id) → MonthlyReport
get_monthly_report_config(guild_id) → MonthlyReportConfig
set_monthly_report_config(guild_id, config) → MonthlyReportConfig
```

---

### Phase 3 — Moyenne priorite

#### Pages existantes a enrichir

**`VoiceChannelsPage.vue`** — Ajouter stats vocales

- Onglet "Statistiques" : salons crees, duree moyenne, pic de participants, top createurs
- Graphique d'evolution dans le temps

**`TicketsPage.vue`** — Ajouter templates

- Bouton "Templates" dans le panneau de reponse
- Liste des templates disponibles avec usage_count
- CRUD templates dans un modal

**`WatchedUsersPage.vue`** — Ajouter historique pseudos

- Tab "Historique des noms" dans le dossier utilisateur
- Timeline des changements de pseudo/username/display_name

**`RolePanelsPage.vue`** — Ajouter roles exclusifs + conditionnels

- Champ "Groupe exclusif" dans l'edition du panel
- Champs conditionnels : roles prerequis, jours min dans le serveur, niveau min

#### Nouveaux composables

| Composable | Description |
|------------|-------------|
| `useVoiceStats` | Stats vocales agregees par guild |
| `useTicketTemplates` | Templates de reponse CRUD |
| `useNameHistory` | Historique des pseudos |

#### Nouveaux types TypeScript

```typescript
interface VoiceChannelStats {
  total_channels_created: number;
  avg_duration_secs: number;
  avg_peak_members: number;
  total_joins: number;
}

interface TicketTemplate {
  id: string;
  guild_id: string;
  name: string;
  content: string;
  category: string;
  usage_count: number;
  created_by: string;
  created_at: string;
}

interface NameHistoryEntry {
  id: string;
  guild_id: string;
  user_id: string;
  old_name: string;
  new_name: string;
  change_type: "nickname" | "username" | "display_name";
  created_at: string;
}
```

#### Nouvelles commandes Tauri (8)

```
get_voice_stats(guild_id) → VoiceChannelStats
get_ticket_templates(guild_id) → Vec<TicketTemplate>
create_ticket_template(...) → TicketTemplate
delete_ticket_template(id) → ()
use_ticket_template(id) → TicketTemplate
get_name_history(guild_id, user_id) → Vec<NameHistoryEntry>
```

---

### Phase 4 — Nice-to-have

#### Pages existantes a enrichir

**`LevelsPage.vue`** — Ajouter badges/achievements

- Onglet "Badges" : liste de tous les badges disponibles avec conditions
- Badges d'un utilisateur dans son profil
- Progression vers le prochain badge

**`ModerationPage.vue`** — Ajouter templates d'avertissement

- Autocomplete sur le champ raison avec les templates
- Gestion des templates dans un modal

**`RolePanelsPage.vue`** — Ajouter reaction roles

- Section "Reaction Roles" a cote des panels boutons
- Config : message_id, emoji, role_id

#### Nouveaux composables

| Composable | Description |
|------------|-------------|
| `useBadges` | Definitions + badges par utilisateur |
| `useWarningTemplates` | Templates d'avertissement CRUD |
| `useReactionRoles` | Reaction roles CRUD |

#### Nouveaux types TypeScript

```typescript
interface BadgeDefinition {
  id: string;
  name: string;
  description: string;
  emoji: string;
  category: "messages" | "vocal" | "levels" | "moderation" | "special";
  condition: { type: string; threshold: number };
}

interface UserBadge {
  badge_id: string;
  awarded_at: string;
  definition: BadgeDefinition;
}

interface WarningTemplate {
  id: string;
  guild_id: string;
  name: string;
  reason: string;
  gravity: "low" | "medium" | "high";
  category: string;
}

interface ReactionRole {
  id: string;
  guild_id: string;
  channel_id: string;
  message_id: string;
  emoji: string;
  role_id: string;
}
```

#### Nouvelles commandes Tauri (8)

```
get_badge_definitions() → Vec<BadgeDefinition>
get_user_badges(guild_id, user_id) → Vec<UserBadge>
get_warning_templates(guild_id) → Vec<WarningTemplate>
create_warning_template(...) → WarningTemplate
delete_warning_template(id) → ()
get_reaction_roles(guild_id) → Vec<ReactionRole>
create_reaction_role(...) → ReactionRole
delete_reaction_role(id) → ()
```

---

### Nouveaux composants reutilisables a creer

Pour eviter la duplication dans les nouvelles pages, ces composants doivent etre crees :

| Composant | Type | Utilise par | Description |
|-----------|------|------------|-------------|
| `CountdownBadge.vue` | Atom | ScheduledActions, TemporaryRoles | Badge avec compte a rebours (ex: "dans 2h 15min") |
| `TimelineEvent.vue` | Molecule | Anomalies, Audit, Strikes | Evenement dans une timeline verticale |
| `ConfigSlider.vue` | Atom | SLA, Strikes, Anomalies | Slider avec label + valeur + unite configurable |
| `StarRating.vue` | Atom | Satisfaction | Affichage et saisie d'etoiles (1-5) |
| `ProgressRing.vue` | Atom | Badges, Streaks | Cercle de progression (%) |
| `StatusTimeline.vue` | Organism | Appeals, Tickets | Timeline d'etapes (pending → reviewing → resolved) |
| `NoteCard.vue` | Molecule | WatchedUsers (notes) | Carte de note avec categorie, auteur, date |
| `StrikeIndicator.vue` | Atom | Infractions, Moderation | Indicateur visuel du nombre de strikes (points colores) |

### Modifications du SidebarNav

Ajouter les nouvelles entrees de navigation :

```
Section Moderation:
  + Strikes (/strikes) — icone: alert-triangle
  + Appels (/appeals) — icone: message-circle
  + Actions programmees (/scheduled-actions) — icone: clock
  + Convocations (/calls) — icone: phone
  + Rapports mensuels IA (/monthly-reports) — icone: bar-chart

Section Securite:
  + Anomalies (/anomalies) — icone: activity

Section Communaute:
  + Roles temporaires (/temporary-roles) — icone: timer
```

### Notifications WebSocket a ajouter

| Evenement | Source | Action desktop |
|-----------|--------|---------------|
| `strike_added` | API (via automod/moderation) | Notification + refresh page strikes |
| `appeal_created` | API (via bot commande) | Notification urgente + compteur sidebar |
| `anomaly_detected` | API (via audit bot) | Notification critique + popup |
| `sla_breach` | Worker (check_sla) | Notification + badge rouge sur tickets |
| `scheduled_action_executed` | Worker | Notification info |
| `temporary_role_expired` | Worker | Notification info |
| `call_opened` | API (via moderation bot) | Notification + refresh page calls |
| `call_closed` | API (via moderation bot) | Notification info |
| `monthly_report_generated` | Worker (analytics) | Notification + affichage du nouveau rapport |

---

### Resume des modifications application desktop

| Phase | Nouvelles pages | Pages enrichies | Composables | Types TS | Commandes Tauri | Composants UI |
|-------|----------------|----------------|-------------|----------|----------------|---------------|
| 1 | 4 (Strikes, Appeals, ScheduledActions, Calls) | 2 (WatchedUsers, Moderation) | 5 | 5 | 16 | 3 (NoteCard, StrikeIndicator, StatusTimeline) |
| 2 | 2 (Anomalies, TemporaryRoles) | 2 (Tickets, Levels) | 5 | 7 | 15 | 2 (CountdownBadge, TimelineEvent) |
| 3 | 0 | 4 (VoiceChannels, Tickets, WatchedUsers, RolePanels) | 3 | 3 | 6 | 0 |
| 4 | 0 | 3 (Levels, Moderation, RolePanels) | 3 | 4 | 8 | 3 (StarRating, ProgressRing, ConfigSlider) |
| **Total** | **5** | **11** | **15** | **18** | **42** | **8** |

### Etat apres implementation

| Categorie | Avant | Apres | Delta |
|-----------|-------|-------|-------|
| Routes/Pages | 26 | 32 | +6 |
| Composables | 28 | 44 | +16 |
| Commandes Tauri | 53 | 98 | +45 |
| Types TypeScript | 50+ | 69+ | +19 |
| Traits Repository (Rust) | 15 | 26 | +11 |
| Composants UI | 19 | 27 | +8 |

---

## Nouveaux bots et extensions de bots existants

Analyse des lacunes de couverture. Certaines fonctionnalites sont mieux integrees dans des bots existants pour eviter les chevauchements, d'autres necessitent un bot dedie.

### Analyse des chevauchements potentiels

Avant de creer un nouveau bot, verification que ca n'empiete pas sur l'existant :

| Fonctionnalite | Pourrait etre un nouveau bot | Mieux dans un bot existant | Raison |
|---------------|------------------------------|---------------------------|--------|
| **Onboarding/Welcome** | Welcome Bot | **Community Bot** (extension) | Le community-bot ecoute deja `guild_member_addition` et gere les auto-roles. L'onboarding est une extension naturelle. |
| **Cache messages supprimes** | Logger Bot | **Audit Bot** (extension) | L'audit-bot ecoute deja `message_delete/update`. Ajouter un cache LRU evite un bot duplique. |
| **Rappels personnels** | Reminder Bot | **Moderation Bot** (extension) | Le moderation-bot gere deja les actions programmees. Ajouter `/remind` est naturel. |
| **Evenements communautaires** | Event Bot | **Nouveau bot (justifie)** | Ecoute `guild_scheduled_event_*` que personne ne couvre. Logique complexe (inscriptions, rappels, recaps). |
| **Concours/Giveaways** | Giveaway Bot | **Nouveau bot (justifie)** | Aucun bot ne gere les concours. Purement interaction-driven, pas de conflit. |
| **Sondages avances** | Poll Bot | **Nouveau bot (justifie)** | Aucun bot ne gere les sondages. Purement interaction-driven, pas de conflit. |

**Resultat : 3 nouveaux bots + 3 extensions de bots existants** (au lieu de 6 nouveaux bots).

### Couverture actuelle des 9 bots

| Responsabilite | Couverture | Bots concernes | Lacunes |
|----------------|-----------|----------------|---------|
| Contenu des messages | 60% | Automod, Image, Ticket | Pas de toxicite contextuelle, pas d'OCR, pas d'analyse GIF |
| Comportement utilisateur | 40% | Security, Stats, Audit | Pas de tracking profil, pas de detection typing, pas de presence |
| Enforcement moderation | 55% | Moderation, Automod | Pas d'escalade, pas de scheduling, pas d'actions en masse |
| Engagement communaute | 40% | Stats, Roles, Voice | Pas d'achievements, pas de retention, pas de gamification |
| Securite avancee | 50% | Security, Audit | Patterns de raid sophistiques, tracking integrations manquant |
| Reponse aux incidents | 20% | Security (slowmode, quarantaine) | Pas de lockdown complet, pas de replay, pas de templates incidents |

### Evenements Discord non couverts par aucun bot

| Evenement | Impact | Interet |
|-----------|--------|---------|
| `auto_moderation_action_execution` | Correlation avec l'automod Discord natif | HIGH |
| `guild_scheduled_event_*` | Gestion des evenements planifies Discord | MEDIUM |
| `integration_create/update/delete` | Tracking des bots/apps ajoutes au serveur | MEDIUM |
| `stage_instance_*` | Monitoring des stage channels (Go Live) | LOW |
| `channel_pins_update` | Tracking des messages epingles | LOW |

---

---

### Extensions de bots existants

Ces fonctionnalites sont integrees dans des bots deja en place pour eviter les doublons d'evenements et la complexite inutile.

#### Extension Community Bot (ex Roles Bot) → Onboarding et retention des membres

**Pourquoi dans le Community Bot** : Il ecoute deja `guild_member_addition` et gere les auto-roles. L'onboarding est la suite logique : apres l'attribution du role, guider le membre.

**Evenements Discord :**
- `guild_member_addition` — declencheur principal
- `interaction_create` — boutons du parcours onboarding
- `message` — messages dans les canaux de presentation
- `guild_member_update` — progression dans les roles

**Logique metier :**

```
Nouveau membre rejoint
  |
  v
1. Message de bienvenue personnalise (DM ou canal dedie)
   - Nom du serveur, nombre de membres, regles resumees
   - Boutons : "Lire les regles", "Se presenter", "Choisir mes roles"
  |
2. Parcours d'onboarding guide (steps configurables per-guild)
   - Etape 1 : Lire et accepter les regles (bouton confirmation)
   - Etape 2 : Se presenter dans #presentations (detection auto du message)
   - Etape 3 : Choisir ses roles dans le panel
   - Etape 4 : Primer message dans un canal general
  |
3. Progression tracking
   - Chaque etape completee = notification au membre
   - Role "Membre verifie" attribue a la fin du parcours
   - Membres qui n'ont pas complete apres X heures → DM de rappel
  |
4. Suivi de retention
   - Tracker quand les nouveaux membres deviennent actifs
   - Tracker quand ils partent (et a quelle etape du parcours)
   - Stats : taux de completion du parcours, taux de retention J7/J30
```

**Configuration per-guild :**

| Parametre | Defaut | Description |
|-----------|--------|-------------|
| `welcome_channel_id` | null | Canal pour les messages de bienvenue |
| `presentation_channel_id` | null | Canal #presentations |
| `verified_role_id` | null | Role attribue apres completion |
| `onboarding_steps` | [] | Etapes du parcours (JSON) |
| `reminder_hours` | 24 | Delai avant rappel DM |
| `welcome_dm_enabled` | true | Envoyer un DM de bienvenue |
| `goodbye_channel_id` | null | Canal pour les messages de depart |

**Endpoints API necessaires :**

| Methode | Route | Description |
|---------|-------|-------------|
| GET | `/api/onboarding/config/{guild_id}` | Config onboarding |
| PUT | `/api/onboarding/config/{guild_id}` | Modifier la config |
| GET | `/api/onboarding/progress/{guild_id}/{user_id}` | Progression d'un membre |
| POST | `/api/onboarding/complete-step` | Marquer une etape comme completee |
| GET | `/api/onboarding/stats/{guild_id}` | Stats de retention et completion |
| GET | `/api/onboarding/incomplete/{guild_id}` | Membres n'ayant pas complete |

**Table SQL :**

```sql
CREATE TABLE onboarding_config (
    guild_id            TEXT PRIMARY KEY,
    welcome_channel_id  TEXT,
    presentation_channel_id TEXT,
    verified_role_id    TEXT,
    steps               JSONB NOT NULL DEFAULT '[]',
    reminder_hours      INT NOT NULL DEFAULT 24,
    welcome_dm_enabled  BOOLEAN NOT NULL DEFAULT TRUE,
    goodbye_channel_id  TEXT,
    enabled             BOOLEAN NOT NULL DEFAULT TRUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE member_onboarding (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    username    TEXT NOT NULL,
    steps_completed JSONB NOT NULL DEFAULT '[]',
    completed   BOOLEAN NOT NULL DEFAULT FALSE,
    completed_at TIMESTAMPTZ,
    joined_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    left_at     TIMESTAMPTZ,
    reminded    BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE(guild_id, user_id)
);
CREATE INDEX idx_onboarding_guild ON member_onboarding(guild_id, completed);
```

---

#### Extension Audit Bot → Cache messages et contenu des suppressions

**Pourquoi dans l'Audit Bot** : Il ecoute deja `message_delete`, `message_update`, `message_delete_bulk`. Ajouter un cache LRU (`DashMap<MessageId, CachedMessage>`, max 10K par guild, ~5 Mo) permet d'enrichir les logs existants sans creer un bot separe.

**Ajouts au code :**
- `message_cache.rs` : `DashMap` avec eviction LRU (taille max configurable via env `MESSAGE_CACHE_SIZE`)
- Sur chaque `message` : stocker `{content, author_id, author_name, attachments, timestamp}` dans le cache
- Sur `message_delete` : chercher dans le cache avant d'envoyer le log. Si trouve, inclure le contenu original.
- Sur `message_update` : comparer l'ancien contenu (cache) avec le nouveau et inclure le diff.
- Sur `message_delete_bulk` : enrichir chaque message avec le contenu cache.

**Nouveau endpoint API :**

| Methode | Route | Description |
|---------|-------|-------------|
| POST | `/api/message-logs` | Log enrichi d'un message supprime/edite |
| GET | `/api/message-logs/{guild_id}` | Historique des messages supprimes/edites |

**Nouvelle table SQL :**

```sql
CREATE TABLE message_logs (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    channel_id      TEXT NOT NULL,
    message_id      TEXT NOT NULL,
    author_id       TEXT NOT NULL,
    author_name     TEXT NOT NULL,
    event_type      TEXT NOT NULL,       -- "delete", "edit", "bulk_delete"
    original_content TEXT,
    new_content     TEXT,
    attachments     JSONB DEFAULT '[]',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_message_logs_guild ON message_logs(guild_id, created_at DESC);
```

---

#### Extension Moderation Bot → Rappels

**Pourquoi dans le Moderation Bot** : Il gere deja les actions programmees (scheduled_actions). Les rappels sont une variante : au lieu d'executer une action Discord, envoyer un DM ou une mention.

**Ajouts :**
- Commande `/remind me <duree> <message>` : rappel personnel par DM
- Commande `/remind here <duree> <message>` : rappel dans le canal
- Commande `/remind list` : rappels actifs
- Commande `/remind cancel <id>` : annuler

**Reutilise la table `scheduled_actions`** avec `action_type = "reminder_dm"` ou `"reminder_channel"`. Le worker existant `execute_scheduled_actions` gere l'envoi.

---

### Nouveaux bots (3 bots justifies, sans chevauchement)

Ces bots ecoutent des evenements ou gerent des fonctionnalites qu'aucun bot existant ne couvre.

#### Bot 1 : Event Bot — Gestion des evenements communautaires

**Pourquoi** : Discord a un systeme d'evenements planifies (Scheduled Events) mais aucun bot ne les exploite. Pas de gestion des inscriptions, des rappels, des recaps post-event.

**Evenements Discord :**
- `guild_scheduled_event_create/update/delete` — lifecycle des events
- `guild_scheduled_event_user_add/remove` — inscriptions
- `interaction_create` — boutons inscription, rappels
- `voice_state_update` — presence dans le vocal pendant l'event

**Logique metier :**

```
Event cree (Discord Scheduled Event ou commande /event create)
  |
  v
1. Annonce automatique dans un canal configure
   - Embed riche : titre, description, date, organisateur
   - Boutons : "S'inscrire", "Rappel 1h avant", "Peut-etre"
  |
2. Gestion des inscriptions
   - Tracking des inscrits avec role temporaire "Participant"
   - Limite de places configurable
   - Liste d'attente si complet
  |
3. Rappels automatiques
   - DM aux inscrits : 24h avant, 1h avant, 10 min avant (configurable)
   - Mention du role "Participant" dans le canal a 10 min
  |
4. Pendant l'event
   - Tracking des presents (qui rejoint le vocal / le stage)
   - Stats en temps reel : nombre de participants
  |
5. Apres l'event
   - Recap automatique : duree, participants, pics
   - Sondage de satisfaction optionnel
   - Retrait du role temporaire
   - Stats envoyees au backend pour analytics
```

**Commandes slash :**

| Commande | Description |
|----------|-------------|
| `/event create <titre> <date> <description> [limite]` | Creer un evenement + annonce |
| `/event list` | Lister les events a venir |
| `/event cancel <event_id>` | Annuler un evenement |
| `/event recap <event_id>` | Generer le recap d'un event passe |
| `/event stats` | Stats globales des events du serveur |

**Endpoints API :**

| Methode | Route | Description |
|---------|-------|-------------|
| POST | `/api/events` | Creer un evenement |
| GET | `/api/events/{guild_id}` | Lister les events |
| PATCH | `/api/events/{id}/cancel` | Annuler |
| POST | `/api/events/{id}/register` | Inscrire un membre |
| DELETE | `/api/events/{id}/register/{user_id}` | Desinscrire |
| GET | `/api/events/{id}/attendees` | Liste des inscrits |
| POST | `/api/events/{id}/recap` | Generer un recap |
| GET | `/api/events/stats/{guild_id}` | Stats events |

**Tables SQL :**

```sql
CREATE TABLE community_events (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    title           TEXT NOT NULL,
    description     TEXT,
    organizer_id    TEXT NOT NULL,
    organizer_name  TEXT NOT NULL,
    channel_id      TEXT,
    voice_channel_id TEXT,
    scheduled_at    TIMESTAMPTZ NOT NULL,
    ends_at         TIMESTAMPTZ,
    max_attendees   INT,
    status          TEXT NOT NULL DEFAULT 'scheduled',
    discord_event_id TEXT,
    announcement_message_id TEXT,
    participant_role_id TEXT,
    actual_attendees INT DEFAULT 0,
    peak_attendees  INT DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE event_registrations (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id    UUID NOT NULL REFERENCES community_events(id),
    user_id     TEXT NOT NULL,
    username    TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'registered',
    reminded    BOOLEAN NOT NULL DEFAULT FALSE,
    attended    BOOLEAN NOT NULL DEFAULT FALSE,
    registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(event_id, user_id)
);
CREATE INDEX idx_event_reg ON event_registrations(event_id, status);
```

**Nouveau job worker :** `event_reminders` dans le moderation-worker (toutes les 5 min) — envoie les rappels DM.

---

#### Bot 2 : Giveaway Bot — Concours et tirages au sort

**Pourquoi** : Les giveaways sont un outil majeur d'engagement communautaire. Aucun bot ne gere les concours, les conditions de participation, les tirages transparents.

**Evenements Discord :**
- `interaction_create` — bouton "Participer", commandes slash
- `message_reaction_add` — participation par reaction (optionnel)

**Logique metier :**

```
/giveaway create <prix> <duree> [conditions]
  |
  v
1. Embed du concours dans le canal courant
   - Prix, duree, conditions, nombre de gagnants
   - Bouton "Participer" (custom_id: giveaway_{id})
   - Compteur de participants en temps reel
  |
2. Conditions de participation (optionnelles, configurables)
   - Avoir un role specifique
   - Etre dans le serveur depuis X jours
   - Avoir un niveau minimum
   - Ne pas avoir de strikes actifs
  |
3. Pendant le concours
   - Chaque clic sur "Participer" → verification des conditions
   - Si eligible : inscription + confirmation ephemere
   - Si non eligible : message d'erreur ephemere avec raison
  |
4. Fin du concours (timer ou commande /giveaway end)
   - Tirage au sort aleatoire parmi les participants eligibles
   - Annonce du/des gagnant(s) dans le canal
   - DM aux gagnants
   - Mise a jour de l'embed original (gagnant affiché)
  |
5. Reroll optionnel
   - /giveaway reroll <giveaway_id> : retirer si le gagnant ne repond pas
```

**Commandes slash :**

| Commande | Description |
|----------|-------------|
| `/giveaway create <prix> <duree> [gagnants] [role_requis] [niveau_min]` | Creer un concours |
| `/giveaway list` | Concours actifs |
| `/giveaway end <id>` | Terminer et tirer au sort |
| `/giveaway reroll <id>` | Re-tirer un gagnant |
| `/giveaway cancel <id>` | Annuler un concours |

**Endpoints API :**

| Methode | Route | Description |
|---------|-------|-------------|
| POST | `/api/giveaways` | Creer |
| GET | `/api/giveaways/{guild_id}` | Lister |
| POST | `/api/giveaways/{id}/enter` | Participer |
| POST | `/api/giveaways/{id}/draw` | Tirer au sort |
| POST | `/api/giveaways/{id}/reroll` | Re-tirer |
| PATCH | `/api/giveaways/{id}/cancel` | Annuler |

**Table SQL :**

```sql
CREATE TABLE giveaways (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    channel_id      TEXT NOT NULL,
    message_id      TEXT,
    prize           TEXT NOT NULL,
    description     TEXT,
    creator_id      TEXT NOT NULL,
    winner_count    INT NOT NULL DEFAULT 1,
    required_role_id TEXT,
    min_level       INT,
    min_server_days INT,
    no_strikes      BOOLEAN NOT NULL DEFAULT FALSE,
    status          TEXT NOT NULL DEFAULT 'active',
    ends_at         TIMESTAMPTZ NOT NULL,
    winners         JSONB DEFAULT '[]',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE giveaway_entries (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    giveaway_id UUID NOT NULL REFERENCES giveaways(id),
    user_id     TEXT NOT NULL,
    username    TEXT NOT NULL,
    entered_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(giveaway_id, user_id)
);
CREATE INDEX idx_giveaway_entries ON giveaway_entries(giveaway_id);
```

**Nouveau job worker :** `end_giveaways` dans le moderation-worker (toutes les 30s) — termine les concours expires.

---

#### Bot 3 : Poll Bot — Sondages avances

**Pourquoi** : Discord a un systeme de sondage natif basique (Poll) mais il ne supporte pas les sondages avances : votes ponderes, sondages anonymes, resultats programmes, quorum.

**Evenements Discord :**
- `interaction_create` — creation + votes par boutons/menus

**Logique metier :**

```
/poll create <question> <options> [duree] [type]
  |
  v
1. Types de sondage :
   - Simple : une seule reponse (boutons)
   - Multiple : plusieurs reponses (checkboxes via menu)
   - Classement : ordonner les options (menu select)
   - Oui/Non : question fermee
   - Anonyme : votes non visibles individuellement
  |
2. Embed du sondage avec boutons/menu
   - Question, options, createur, temps restant
   - Barre de progression en temps reel par option
   - Compteur de votants
  |
3. Options avancees :
   - Quorum minimum (X votes pour que le resultat soit valide)
   - Role requis pour voter
   - Duree configurable (min 1 min, max 30 jours)
   - Resultats caches jusqu'a la fin (mode "enveloppe")
  |
4. Fin du sondage
   - Affichage des resultats avec graphique
   - DM au createur si quorum non atteint
   - Archivage en BDD
```

**Commandes slash :**

| Commande | Description |
|----------|-------------|
| `/poll create <question> <option1> <option2> [option3-10] [duree] [type]` | Creer un sondage |
| `/poll end <id>` | Terminer manuellement |
| `/poll results <id>` | Voir les resultats (si autorise) |
| `/poll list` | Sondages actifs du serveur |

**Endpoints API :**

| Methode | Route | Description |
|---------|-------|-------------|
| POST | `/api/polls` | Creer |
| GET | `/api/polls/{guild_id}` | Lister |
| POST | `/api/polls/{id}/vote` | Voter |
| GET | `/api/polls/{id}/results` | Resultats |
| PATCH | `/api/polls/{id}/end` | Terminer |

**Table SQL :**

```sql
CREATE TABLE polls (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id        TEXT NOT NULL,
    channel_id      TEXT NOT NULL,
    message_id      TEXT,
    question        TEXT NOT NULL,
    options         JSONB NOT NULL,
    poll_type       TEXT NOT NULL DEFAULT 'simple',
    creator_id      TEXT NOT NULL,
    required_role_id TEXT,
    quorum          INT,
    anonymous       BOOLEAN NOT NULL DEFAULT FALSE,
    hidden_results  BOOLEAN NOT NULL DEFAULT FALSE,
    status          TEXT NOT NULL DEFAULT 'active',
    ends_at         TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE poll_votes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    poll_id     UUID NOT NULL REFERENCES polls(id),
    user_id     TEXT NOT NULL,
    choices     JSONB NOT NULL,
    voted_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(poll_id, user_id)
);
```

---

---

### Resume final

#### 3 extensions de bots existants (pas de nouveau token Discord)

| Bot existant | Extension | Ajouts |
|-------------|-----------|--------|
| **Community Bot** (ex Roles) | Onboarding + retention | 2 tables, 6 endpoints, 1 job worker |
| **Audit Bot** | Cache messages (LRU) + logs enrichis | 1 table, 2 endpoints, DashMap en memoire |
| **Moderation Bot** | Rappels (`/remind`) | Reutilise `scheduled_actions`, 0 nouvelle table |

#### 3 nouveaux bots (avec justification de non-chevauchement)

| Bot | Cas d'usage | Pourquoi un nouveau bot | Priorite | Tables | Endpoints | Job worker |
|-----|------------|------------------------|----------|--------|-----------|------------|
| **Event Bot** | Evenements communautaires | Ecoute `guild_scheduled_event_*` (aucun bot ne couvre) + logique complexe inscriptions/rappels/recaps | HIGH | 2 | 8 | 1 |
| **Giveaway Bot** | Concours et tirages au sort | Aucun bot ne gere les concours. Purement interaction-driven | MEDIUM | 2 | 6 | 1 |
| **Poll Bot** | Sondages avances | Aucun bot ne gere les sondages. Purement interaction-driven | MEDIUM | 2 | 5 | 1 |

#### Nouveaux tokens Discord necessaires (3 au lieu de 6)

```env
EVENT_DISCORD_TOKEN=...
GIVEAWAY_DISCORD_TOKEN=...
POLL_DISCORD_TOKEN=...
```

#### Impact sur l'architecture

| Composant | Avant | Apres | Delta |
|-----------|-------|-------|-------|
| Bots Discord | 9 | 12 | +3 |
| Tables PostgreSQL | 28 | 35 | +7 |
| Endpoints API | 62+ | 83+ | +21 |
| Jobs worker | 5 | 8 | +3 |
| Tokens Discord | 9 | 12 | +3 |
| Services docker-compose | 15 | 18 | +3 |

---

## Design des embeds Discord (cards)

Tous les bots utilisent maintenant des embeds riches uniformes via le helper partage `bots/shared/src/embeds.rs`. Implementation terminee.

### Etat actuel (IMPLEMENTE)

| Bot | Replies utilisateur | Logs Discord | DMs | Statut |
|-----|:--:|:--:|:--:|--------|
| **Automod** | Embed riche | Embed riche | - | ✅ Fait |
| **Moderation** | Embed riche | - | Embed riche | ✅ Fait |
| **Security** | Embed riche | - | Embed riche (captcha) | ✅ Fait |
| **Progression** | Embed riche | - | - | ✅ Fait (level up ameliore) |
| **Ticket** | Components | - | - | ✅ Fait (auto-close embed) |
| **Image** | Embed riche | - | - | ✅ Fait |
| **Voice** | Embed riche | - | - | ✅ Deja en embeds |
| **Audit** | Backend only | - | - | N/A |
| **Community** | Embed riche | - | - | ✅ Fait |

### Palette de couleurs unifiee

Toutes les cards utilisent cette palette (coherente avec l'app desktop) :

| Couleur | Hex | Usage |
|---------|-----|-------|
| Blurple (accent) | `0x5865F2` | Info, stats, niveaux, roles |
| Vert (success) | `0x57F287` | Actions reussies, bienvenue, verification OK |
| Jaune (warning) | `0xFEE75C` | Avertissements, warn, rappels |
| Orange (moderation) | `0xF97316` | Mute, suppression, sanctions legeres |
| Rouge (danger) | `0xED4245` | Ban, raid, contenu illicite |
| Rouge sombre | `0xDC2626` | Ban permanent, alerte critique |
| Gris (neutre) | `0x95A5A6` | Messages systeme, infos neutres |

### Structure commune des embeds

Tous les embeds suivent ce template de base :

```
┌─────────────────────────────────────────────────┐
│ [barre couleur en haut]                         │
│                                                 │
│ 🤖 Bot Name — Titre de l'action    [thumbnail] │
│                                                 │
│ 👤 Utilisateur    💬 Salon    ⚙️ Action         │
│ <@user>           <#channel>  Badge couleur     │
│                                                 │
│ 📝 Raison / Details                             │
│ Le texte de la raison ou du detail...           │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel • Aujourd'hui a 14h35       ID: 12345  │
└─────────────────────────────────────────────────┘
```

Regles :
- Toujours un **footer** avec "Sentinel" + timestamp
- Toujours une **couleur** selon le type d'action
- **Thumbnail** = avatar de l'utilisateur concerne (quand applicable)
- **Fields inline** (3 par ligne) pour les donnees cles
- **Emojis** en prefixe des noms de fields

---

### Cards par bot

#### Automod Bot — Replies utilisateur (actuellement plain text)

**Card Warn :**
```
┌─ JAUNE (0xFEE75C) ─────────────────────────────┐
│ ⚠️ Avertissement AutoMod                [avatar]│
│                                                 │
│ 👤 Utilisateur    💬 Salon    🔍 Detection      │
│ <@user>           <#general>  Spam              │
│                                                 │
│ 📝 Raison                                       │
│ Repetition excessive de caracteres              │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel AutoMod • 30/03/2026 14h35             │
└─────────────────────────────────────────────────┘
```

**Card Delete :**
```
┌─ ORANGE (0xF97316) ─────────────────────────────┐
│ 🗑️ Message supprime                     [avatar]│
│                                                 │
│ 👤 Utilisateur    💬 Salon    🔍 Detection      │
│ <@user>           <#general>  Insulte + Spam    │
│                                                 │
│ 📝 Raison                                       │
│ Contenu offensant detecte                       │
│                                                 │
│ 📩 Message original                             │
│ ``` le contenu du message supprime ```          │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel AutoMod • 30/03/2026 14h35             │
└─────────────────────────────────────────────────┘
```

**Card Mute :**
```
┌─ ROUGE (0xED4245) ──────────────────────────────┐
│ 🔇 Mute automatique (10 min)            [avatar]│
│                                                 │
│ 👤 Utilisateur    💬 Salon    🔍 Detections     │
│ <@user>           <#general>  Phishing + Lien   │
│                                                 │
│ 📝 Raison                                       │
│ Lien de phishing detecte                        │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel AutoMod • 30/03/2026 14h35             │
└─────────────────────────────────────────────────┘
```

---

#### Moderation Bot — Toutes les commandes (actuellement plain text)

**Card /warn :**
```
┌─ JAUNE / ORANGE / ROUGE (selon gravite) ────────┐
│ ⚠️ Avertissement (medium)                [avatar]│
│                                                  │
│ 👤 Cible          👮 Moderateur   📊 Gravite     │
│ <@target>         <@mod>          🟠 Moyen       │
│                                                  │
│ 📝 Raison                                        │
│ Comportement irrespectueux envers les membres    │
│                                                  │
│ 📋 Historique : 2 warns · 0 mutes · 0 bans      │
│                                                  │
│ ──────────────────────────────────────────────── │
│ Sentinel Moderation • 30/03/2026 14h35           │
└──────────────────────────────────────────────────┘
```

Couleur selon gravite : low=`0xFEE75C`, medium=`0xF97316`, high=`0xED4245`

**Card /mute :**
```
┌─ ORANGE (0xF97316) ─────────────────────────────┐
│ 🔇 Mute (30 minutes)                    [avatar]│
│                                                 │
│ 👤 Cible          👮 Moderateur   ⏱️ Duree      │
│ <@target>         <@mod>          30 min        │
│                                                 │
│ 📝 Raison                                       │
│ Spam de mentions dans #general                  │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Moderation • 30/03/2026 14h35          │
└─────────────────────────────────────────────────┘
```

**Card /ban :**
```
┌─ ROUGE SOMBRE (0xDC2626) ───────────────────────┐
│ 🔨 Bannissement (permanent)              [avatar]│
│                                                  │
│ 👤 Cible          👮 Moderateur   ⏱️ Duree       │
│ <@target>         <@mod>          Permanent      │
│                                                  │
│ 📝 Raison                                        │
│ Harcelement repete malgre 3 avertissements       │
│                                                  │
│ 📋 Historique : 3 warns · 1 mute · 1 ban        │
│                                                  │
│ ──────────────────────────────────────────────── │
│ Sentinel Moderation • 30/03/2026 14h35           │
└──────────────────────────────────────────────────┘
```

**Card /history :**
```
┌─ BLURPLE (0x5865F2) ────────────────────────────┐
│ 📋 Historique de @username               [avatar]│
│                                                 │
│ 🟡 Warns          🔇 Mutes       🔨 Bans       │
│ 3                  1              0              │
│                                                 │
│ 📜 Dernieres actions                            │
│ 1. 🟠 **warn** — Spam (il y a 2j)              │
│ 2. 🟡 **warn** — Insulte (il y a 5j)           │
│ 3. 🔇 **mute** — Flood (il y a 1 sem)          │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Moderation • 30/03/2026 14h35          │
└─────────────────────────────────────────────────┘
```

**Card /unmute et /unban :**
```
┌─ VERT (0x57F287) ───────────────────────────────┐
│ ✅ Unmute / Unban                        [avatar]│
│                                                 │
│ 👤 Cible          👮 Moderateur                  │
│ <@target>         <@mod>                         │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Moderation • 30/03/2026 14h35          │
└─────────────────────────────────────────────────┘
```

**DM a l'utilisateur (meme style, couleur + raison) :**
```
┌─ ORANGE (0xF97316) ─────────────────────────────┐
│ 🔇 Vous avez ete mute sur **MonServeur**        │
│                                                 │
│ ⏱️ Duree          📊 Gravite                     │
│ 30 minutes        Moyen                          │
│                                                 │
│ 📝 Raison                                       │
│ Spam de mentions dans #general                  │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel • MonServeur                           │
└─────────────────────────────────────────────────┘
```

---

#### Security Bot — Alertes (actuellement plain text)

**Card Raid detecte :**
```
┌─ ROUGE (0xED4245) ──────────────────────────────┐
│ 🚨 ALERTE RAID DETECTE                          │
│                                                 │
│ 👥 Joins rapides   ⏱️ Fenetre    🛡️ Actions     │
│ 15 membres         10 secondes   Voir ci-dessous│
│                                                 │
│ ⚡ Actions automatiques executees :              │
│ • Verification serveur → Niveau maximum         │
│ • Slowmode active (10s) sur tous les canaux     │
│ • Quarantaine activee pour les nouveaux          │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Security • 30/03/2026 14h35            │
└─────────────────────────────────────────────────┘
```

**Card Compte suspect :**
```
┌─ JAUNE (0xFEE75C) ─────────────────────────────┐
│ ⚠️ Compte suspect detecte               [avatar]│
│                                                 │
│ 👤 Membre         📅 Age du compte               │
│ <@user>           2 heures                       │
│                                                 │
│ 🛡️ Action                                       │
│ Quarantaine + captcha envoye en DM              │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Security • 30/03/2026 14h35            │
└─────────────────────────────────────────────────┘
```

**Card Captcha (DM) :**
```
┌─ BLURPLE (0x5865F2) ────────────────────────────┐
│ 🛡️ Verification requise                         │
│                                                 │
│ Bienvenue sur **MonServeur** !                  │
│                                                 │
│ Pour acceder au serveur, veuillez confirmer     │
│ que vous etes humain en cliquant le bouton      │
│ ci-dessous.                                     │
│                                                 │
│ ⏱️ Vous avez 5 minutes pour verifier.           │
│                                                 │
│ [✅ Je suis humain]  ← bouton                   │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Security • MonServeur                  │
└─────────────────────────────────────────────────┘
```

---

#### Progression Bot — Ameliorations (deja des embeds)

Les embeds existants sont bons. Ameliorations a faire :

**Card /level avec barre de progression visuelle (emojis) :**
```
┌─ BLURPLE (0x5865F2) ────────────────────────────┐
│ ✨ Niveau de @username                   [avatar]│
│                                                 │
│ 🏆 Niveau         ⭐ XP Total    🔥 Streak     │
│ 12                 4,530 XP      7 jours (x2)  │
│                                                 │
│ 📊 Progression vers niveau 13                   │
│ ████████░░░░░░░░ 530 / 1000 XP (53%)           │
│                                                 │
│ 🎖️ Badges : 🏅🎯🔥                              │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Progression • 30/03/2026 14h35         │
└─────────────────────────────────────────────────┘
```

**Card Level Up (annonce dans le canal) :**
```
┌─ VERT (0x57F287) ───────────────────────────────┐
│ 🎉 LEVEL UP !                            [avatar]│
│                                                 │
│ <@user> est maintenant **niveau 12** !          │
│                                                 │
│ 🏆 Niveau 12      ⭐ 4,530 XP    🔥 x2 streak  │
│                                                 │
│ 🎁 Recompense : role @Habitue attribue          │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Progression • 30/03/2026 14h35         │
└─────────────────────────────────────────────────┘
```

**Card /stats top (leaderboard) amelioree :**
```
┌─ JAUNE (0xFEE75C) ─────────────────────────────┐
│ 🏆 Classement — MonServeur                      │
│                                                 │
│ 🥇 @Alice    — 12,340 msgs · 45h vocal         │
│ 🥈 @Bob      —  8,210 msgs · 32h vocal         │
│ 🥉 @Charlie  —  6,890 msgs · 28h vocal         │
│  4. @David   —  5,120 msgs · 15h vocal         │
│  5. @Eve     —  4,800 msgs · 12h vocal         │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Progression • Top 5 sur 30 jours       │
└─────────────────────────────────────────────────┘
```

---

#### Image Bot — Detections (actuellement plain text)

**Card Image supprimee :**
```
┌─ ROUGE (0xED4245) ──────────────────────────────┐
│ 🖼️ Image supprimee                      [avatar]│
│                                                 │
│ 👤 Utilisateur    💬 Salon    🏷️ Detection      │
│ <@user>           <#general>  NSFW (92%)        │
│                                                 │
│ 📝 Raison                                       │
│ Contenu NSFW detecte par l'IA (confiance: 92%)  │
│                                                 │
│ ⚙️ Action : Message supprime                    │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Image • 30/03/2026 14h35               │
└─────────────────────────────────────────────────┘
```

**Card Image — API indisponible (fallback) :**
```
┌─ ORANGE (0xF97316) ─────────────────────────────┐
│ ⚠️ Image supprimee preventivement       [avatar]│
│                                                 │
│ 👤 Utilisateur    💬 Salon                       │
│ <@user>           <#general>                     │
│                                                 │
│ 📝 Raison                                       │
│ Verification impossible (API indisponible).     │
│ L'image a ete supprimee par precaution.         │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Image • 30/03/2026 14h35               │
└─────────────────────────────────────────────────┘
```

---

#### Community Bot (ex Roles) — Reponses (actuellement plain text)

**Card Role attribue :**
```
┌─ VERT (0x57F287) ───────────────────────────────┐
│ ✅ Role attribue                                 │
│                                                 │
│ Le role **@Gamer** vous a ete attribue.         │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Community                              │
└─────────────────────────────────────────────────┘
```

**Card Role retire :**
```
┌─ GRIS (0x95A5A6) ──────────────────────────────┐
│ ↩️ Role retire                                   │
│                                                 │
│ Le role **@Gamer** vous a ete retire.           │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Community                              │
└─────────────────────────────────────────────────┘
```

**Card Bienvenue (onboarding) :**
```
┌─ BLURPLE (0x5865F2) ────────────────────────────┐
│ 👋 Bienvenue sur MonServeur !            [avatar]│
│                                                 │
│ Salut <@user> ! Tu es notre 1,234e membre !     │
│                                                 │
│ 📋 Pour commencer :                             │
│ 1. Lis les regles dans <#regles>                │
│ 2. Presente-toi dans <#presentations>           │
│ 3. Choisis tes roles dans <#roles>              │
│                                                 │
│ [📋 Lire les regles] [👋 Se presenter]          │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Community • MonServeur                 │
└─────────────────────────────────────────────────┘
```

**Card Au revoir :**
```
┌─ GRIS (0x95A5A6) ──────────────────────────────┐
│ 👋 Depart                                       │
│                                                 │
│ **@username** a quitte le serveur.              │
│ Etait membre depuis 45 jours.                   │
│ Parcours onboarding : complete ✅               │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Community • 30/03/2026 14h35           │
└─────────────────────────────────────────────────┘
```

---

#### Voice Bot — Notifications (actuellement plain text)

**Card Salon cree :**
```
┌─ BLURPLE (0x5865F2) ────────────────────────────┐
│ 🎙️ Salon vocal cree                             │
│                                                 │
│ 👤 Proprietaire   🔊 Salon                       │
│ <@user>           Gaming Squad                   │
│                                                 │
│ 🔒 Prive : Non   👥 Limite : 10                 │
│                                                 │
│ [✏️ Renommer] [🔒 Verrouiller] [❌ Fermer]      │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Voice                                  │
└─────────────────────────────────────────────────┘
```

**Card Vote Kick :**
```
┌─ ORANGE (0xF97316) ─────────────────────────────┐
│ 🗳️ Vote Kick en cours                           │
│                                                 │
│ 👤 Cible : <@target>                            │
│ 🗳️ Votes : 3/5 (majorite requise)              │
│ ████████░░░░░░░░ 60%                            │
│ ⏱️ Expire dans 2 minutes                        │
│                                                 │
│ [👍 Voter pour] [👎 Voter contre]               │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Voice                                  │
└─────────────────────────────────────────────────┘
```

---

#### Ticket Bot — Notifications systeme (actuellement plain text)

**Card Ticket cree (dans le canal ticket) :**
```
┌─ BLURPLE (0x5865F2) ────────────────────────────┐
│ 🎫 Ticket #1234 — Probleme avec un membre       │
│                                                 │
│ 👤 Auteur         📂 Categorie   🔴 Priorite    │
│ <@user>           Membre         Haute          │
│                                                 │
│ 📝 Description                                  │
│ Un membre m'insulte en DM depuis 2 jours...     │
│                                                 │
│ [💬 Repondre] [👥 Inviter] [🔊 Appel] [❌ Fermer]│
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Tickets • Ouvert le 30/03/2026 14h35   │
└─────────────────────────────────────────────────┘
```

**Card Ticket ferme :**
```
┌─ VERT (0x57F287) ───────────────────────────────┐
│ ✅ Ticket #1234 ferme                            │
│                                                 │
│ 📊 Resume                                       │
│ Duree : 2h 15min · Messages : 12 · Staff : @mod│
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Tickets • Ferme le 30/03/2026 16h50    │
└─────────────────────────────────────────────────┘
```

**Card Auto-fermeture :**
```
┌─ GRIS (0x95A5A6) ──────────────────────────────┐
│ 🕐 Ticket ferme automatiquement                 │
│                                                 │
│ Ce ticket a ete ferme apres 7 jours             │
│ d'inactivite.                                   │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Tickets • 30/03/2026 14h35             │
└─────────────────────────────────────────────────┘
```

---

### Nouveaux bots — Cards a prevoir

#### Event Bot

**Card Annonce evenement :**
```
┌─ BLURPLE (0x5865F2) ────────────────────────────┐
│ 📅 Soiree Gaming — Tournoi Valorant             │
│                                                 │
│ 🗓️ Date            ⏰ Heure      👥 Places      │
│ Samedi 5 avril     20h00         0/16           │
│                                                 │
│ 📝 Description                                  │
│ Tournoi Valorant en equipes de 5. Inscrivez-    │
│ vous et soyez prets 10 min avant !              │
│                                                 │
│ 👤 Organise par <@organizer>                    │
│                                                 │
│ [✅ S'inscrire] [🔔 Rappel 1h avant] [❔ Peut-etre]│
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Events                                 │
└─────────────────────────────────────────────────┘
```

**Card Rappel evenement (DM) :**
```
┌─ JAUNE (0xFEE75C) ─────────────────────────────┐
│ 🔔 Rappel : Soiree Gaming dans 1 heure !       │
│                                                 │
│ 📅 Samedi 5 avril a 20h00                       │
│ 🔊 Salon vocal : #tournoi-valorant              │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Events • MonServeur                    │
└─────────────────────────────────────────────────┘
```

#### Giveaway Bot

**Card Concours :**
```
┌─ JAUNE (0xFEE75C) ─────────────────────────────┐
│ 🎉 GIVEAWAY — Nitro 1 mois                     │
│                                                 │
│ 🎁 Prix : Discord Nitro (1 mois)               │
│ 🏆 Gagnants : 1                                 │
│ ⏰ Fin : dans 2 jours                           │
│ 👥 Participants : 42                             │
│                                                 │
│ 📋 Conditions :                                 │
│ • Avoir le role @Membre                         │
│ • Etre dans le serveur depuis 7+ jours          │
│                                                 │
│ [🎉 Participer]                                 │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Giveaway • Fin le 02/04/2026 14h35     │
└─────────────────────────────────────────────────┘
```

**Card Gagnant :**
```
┌─ VERT (0x57F287) ───────────────────────────────┐
│ 🎊 GIVEAWAY TERMINE                             │
│                                                 │
│ 🎁 Prix : Discord Nitro (1 mois)               │
│ 🏆 Gagnant : <@winner> 🎉                       │
│ 👥 Participants : 42                             │
│                                                 │
│ Felicitations ! Le gagnant a ete contacte en DM.│
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Giveaway • 02/04/2026 14h35            │
└─────────────────────────────────────────────────┘
```

#### Poll Bot

**Card Sondage :**
```
┌─ BLURPLE (0x5865F2) ────────────────────────────┐
│ 📊 Quel jeu pour la prochaine soiree ?          │
│                                                 │
│ 🎮 Valorant     ████████████░░░░  45% (18)     │
│ 🎮 League       ██████░░░░░░░░░░  25% (10)     │
│ 🎮 Minecraft    ████████░░░░░░░░  30% (12)     │
│                                                 │
│ 👥 40 votes · ⏰ Fin dans 23h                    │
│                                                 │
│ [🎮 Valorant] [🎮 League] [🎮 Minecraft]       │
│                                                 │
│ ─────────────────────────────────────────────── │
│ Sentinel Poll • Cree par @organizer             │
└─────────────────────────────────────────────────┘
```

---

### Implementation : helper partage pour les embeds (IMPLEMENTE)

Le helper partage est en place dans `bots/shared/src/embeds.rs`. Tous les bots l'utilisent.

**`bots/shared/src/embeds.rs`** — Builder d'embeds unifie :

```rust
// Couleurs standardisees
pub const COLOR_INFO: u32     = 0x5865F2; // Blurple
pub const COLOR_SUCCESS: u32  = 0x57F287; // Vert
pub const COLOR_WARNING: u32  = 0xFEE75C; // Jaune
pub const COLOR_MODERATE: u32 = 0xF97316; // Orange
pub const COLOR_DANGER: u32   = 0xED4245; // Rouge
pub const COLOR_CRITICAL: u32 = 0xDC2626; // Rouge sombre
pub const COLOR_NEUTRAL: u32  = 0x95A5A6; // Gris

// Builder avec footer Sentinel + timestamp automatique
pub fn sentinel_embed(title: &str, color: u32) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .color(color)
        .footer(CreateEmbedFooter::new("Sentinel"))
        .timestamp(Timestamp::now())
}

// Variantes pre-configurees
pub fn warn_embed(title: &str) -> CreateEmbed { sentinel_embed(title, COLOR_WARNING) }
pub fn success_embed(title: &str) -> CreateEmbed { sentinel_embed(title, COLOR_SUCCESS) }
pub fn danger_embed(title: &str) -> CreateEmbed { sentinel_embed(title, COLOR_DANGER) }
pub fn info_embed(title: &str) -> CreateEmbed { sentinel_embed(title, COLOR_INFO) }
pub fn moderate_embed(title: &str) -> CreateEmbed { sentinel_embed(title, COLOR_MODERATE) }
pub fn neutral_embed(title: &str) -> CreateEmbed { sentinel_embed(title, COLOR_NEUTRAL) }
```

Chaque bot importe et utilise ces helpers au lieu de construire ses embeds manuellement. Garantit la coherence visuelle sur tous les bots.
