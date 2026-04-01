# Systeme de Surveillance Active

## Objectif

Mettre en place un systeme de surveillance complet qui permet de suivre en temps reel les moindres faits et gestes d'un utilisateur sur le serveur Discord.

## Deux modes de surveillance

### 1. Surveillance manuelle
- Un administrateur choisit de surveiller un membre specifique
- Ajoute via l'application bureau (bouton "+ Surveiller un membre")
- Table `manual_watched_users` (deja implementee)
- Le membre n'a pas forcement d'infraction

### 2. Surveillance automatique
- Declenchee automatiquement quand un membre recoit une infraction (warn, mute, ban)
- Basee sur les donnees existantes de la table `infractions`
- Deja en place via la requete SQL de watched_users

## Tabs dans l'application bureau

La page Surveillance aura 2 onglets :
- **Surveillance manuelle** — membres choisis par les admins
- **Surveillance auto (infractions)** — membres avec des infractions

## Tracking en temps reel

### Evenements a capturer

| Categorie | Evenements | Source |
|-----------|-----------|--------|
| **Messages** | Envoye, edite, supprime | audit-bot |
| **Messages** | Detection copier-coller / spam repetitif | automod-bot + nouveau check |
| **Vocal** | Rejoint, quitte, duree, salon | stats-bot + voice-bot |
| **Salons** | Creation, modification, suppression | audit-bot |
| **Profil** | Changement pseudo, avatar, bio | audit-bot |
| **Roles** | Ajout, retrait | audit-bot |
| **Moderation** | Sanctions recues (warn, mute, ban, kick) | moderation-bot |
| **Invitations** | Creation, utilisation | audit-bot |

### Architecture

```
Discord Events
     |
     v
+------------------+
|   audit-bot      |  (capture tous les evenements)
|   automod-bot    |  (detection contenu)
|   stats-bot      |  (activite messages/vocal)
|   moderation-bot |  (sanctions)
+------------------+
     |
     v  POST /api/user-activity
+------------------+
|   API Backend    |
|   user_activity  |  (nouvelle table)
|   _log           |
+------------------+
     |
     v  GET /api/user-activity/{guild_id}/{user_id}
+------------------+
|   Desktop App    |
|   Timeline       |  (fil d'activite en temps reel)
|   d'activite     |
+------------------+
```

## Base de donnees

### Nouvelle table : `user_activity_log`

```sql
CREATE TABLE user_activity_log (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id    TEXT NOT NULL,
    user_id     TEXT NOT NULL,
    event_type  TEXT NOT NULL,       -- message_sent, message_edited, message_deleted,
                                    -- voice_join, voice_leave, role_added, role_removed,
                                    -- nickname_changed, channel_created, spam_detected, etc.
    channel_id  TEXT,                -- salon concerne (si applicable)
    channel_name TEXT,
    content     TEXT,                -- contenu du message ou details de l'evenement
    metadata    JSONB DEFAULT '{}',  -- donnees supplementaires (ancien pseudo, ancien message, etc.)
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_activity_guild_user ON user_activity_log (guild_id, user_id);
CREATE INDEX idx_user_activity_created ON user_activity_log (created_at);
CREATE INDEX idx_user_activity_type ON user_activity_log (event_type);
```

### Politique de retention

- Garder les logs X jours (configurable, defaut 30)
- Nettoyage automatique par le moderation-worker
- Archivage optionnel avant suppression

## API Endpoints

| Methode | Endpoint | Description |
|---------|----------|-------------|
| `POST` | `/api/user-activity` | Enregistrer un evenement |
| `GET` | `/api/user-activity/{guild_id}/{user_id}` | Timeline d'un utilisateur |
| `GET` | `/api/user-activity/{guild_id}/{user_id}/stats` | Resume statistique |
| `DELETE` | `/api/user-activity/{guild_id}/{user_id}` | Purger l'historique |

### Filtrage

```
GET /api/user-activity/{guild_id}/{user_id}?event_type=message_sent&limit=50&offset=0
```

## Application Bureau — Interface

### Page Surveillance (tabs)

```
[Surveillance manuelle] [Infractions]

+-- Liste utilisateurs (gauche) --+-- Dossier / Timeline (droite) --+
|                                 |                                  |
|  [Avatar] Username              |  Timeline d'activite :           |
|  Risk: Critique                 |                                  |
|  Raison: Comportement suspect   |  14:32 - Message envoye          |
|                                 |    #general : "salut les gens"   |
|  [Avatar] Username2             |                                  |
|  Risk: Eleve                    |  14:28 - Message supprime        |
|  Infractions: 3 warns           |    #discussion : [contenu]       |
|                                 |                                  |
|                                 |  14:15 - Rejoint vocal           |
|                                 |    Salon de Darkponey             |
|                                 |                                  |
|                                 |  13:50 - Pseudo change           |
|                                 |    "ancien" -> "nouveau"          |
|                                 |                                  |
|                                 |  13:30 - Warn recu               |
|                                 |    Raison: spam                   |
+---------------------------------+----------------------------------+
```

### Fonctionnalites de la timeline

- Filtrage par type d'evenement (messages, vocal, moderation, etc.)
- Recherche dans le contenu des messages
- Detection automatique des patterns :
  - Messages identiques repetes (copier-coller)
  - Frequence anormale de messages
  - Changements de pseudo frequents
- Export du rapport de surveillance (PDF/JSON)
- Indicateurs visuels par couleur selon la gravite

## Modifications des bots

### audit-bot (principal)
- Pour chaque evenement capture, verifier si l'utilisateur est dans la liste des surveilles
- Si oui, envoyer `POST /api/user-activity` avec les details
- Ajouter un check pour les messages en double (hash du contenu)

### automod-bot
- Quand un message est flag (spam, insulte, etc.), enregistrer dans user_activity_log

### moderation-bot
- Quand une sanction est appliquee, enregistrer dans user_activity_log

### stats-bot
- Quand un utilisateur surveille envoie un message ou rejoint un vocal, enregistrer

## Priorite d'implementation

### Phase 1 — Base (HIGH)
1. Migration : creer la table `user_activity_log`
2. API : endpoint POST + GET avec filtrage
3. audit-bot : enregistrer messages, edits, deletes pour les surveilles
4. Desktop : tabs + timeline basique

### Phase 2 — Enrichissement (MEDIUM)
5. audit-bot : profil, roles, invitations
6. stats-bot : activite vocale
7. Detection patterns (copier-coller, frequence)
8. Resume statistique (endpoint stats)

### Phase 3 — Avance (LOW)
9. Alertes en temps reel (Redis → desktop via WebSocket)
10. Export rapport
11. Retention automatique + archivage
12. Dashboard surveillance avec graphiques
