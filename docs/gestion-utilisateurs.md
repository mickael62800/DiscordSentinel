# Gestion des utilisateurs — DiscordSentinel Desktop

## Objectif

Ajouter une page "Membres" dans l'application bureau qui permet de visualiser tous les membres d'un serveur Discord, consulter leur profil complet avec tout leur historique, et effectuer des actions de moderation directement depuis l'app.

---

## Fonctionnalites

### Liste des membres

- Afficher tous les membres du serveur selectionne
- Colonnes : avatar, pseudo, roles, date d'arrivee, points de conduite, nombre d'infractions
- Tri par : nom, date d'arrivee, points de conduite, nombre d'infractions
- Recherche par pseudo
- Filtres : par role, par statut (en ligne/hors ligne), par niveau de points (critique/attention/ok)
- Badge couleur selon les points de conduite (vert/orange/rouge/noir)

### Fiche membre (clic sur un membre)

**Informations generales :**
- Avatar + pseudo + discriminateur
- ID Discord
- Date de creation du compte
- Date d'arrivee sur le serveur
- Roles attribues
- Statut actuel (en ligne, absent, ne pas deranger, hors ligne)

**Points de conduite :**
- Points actuels / max
- Barre de progression coloree
- Historique des mouvements de points (tableau avec date, delta, raison)

**Historique des infractions :**
- Liste de toutes les infractions (spam, insulte, lien)
- Date, type, score, action prise, contenu du message

**Historique de moderation :**
- Warns recus
- Mutes recus (avec duree)
- Bans recus
- Qui a effectue l'action + raison

**Statistiques :**
- Nombre total de messages
- Temps en vocal
- Derniere activite

### Actions rapides (depuis la fiche membre)

- **Avertir** (warn) — envoie un warn via le moderation-bot
- **Mute** — timeout l'utilisateur (duree configurable)
- **Ban** — bannir l'utilisateur (avec raison)
- **Ajuster les points** — ajouter ou retirer des points de conduite manuellement
- **Voir les salons vocaux** — dans quels salons vocaux temporaires il est/a ete

---

## Architecture technique

### API — Nouveaux endpoints

```
GET  /api/members/{guild_id}                    — Liste des membres (depuis le cache Discord)
GET  /api/members/{guild_id}/{user_id}          — Profil complet d'un membre
GET  /api/members/{guild_id}/{user_id}/summary  — Resume : infractions + moderation + points + stats
```

Le endpoint `/api/members/{guild_id}` necessite que les bots remontent la liste des membres au demarrage et a chaque changement (guild_member_add, guild_member_remove). Alternatives :

**Option A — Scan via bot** : un bot (security-bot ou un nouveau members-bot) envoie la liste des membres a l'API au demarrage, et met a jour en temps reel via les events Discord.

**Option B — Appel Discord API direct** : l'API backend appelle directement l'API Discord pour recuperer les membres (necessite un token bot). Plus simple mais couple l'API a Discord.

**Option C — Cache local dans l'app desktop** : l'app desktop recupere les membres via un bot qui expose la liste, et croise avec les donnees de l'API (infractions, points, stats). Pas de nouvelle table cote API.

**Recommandation** : Option A — le security-bot remonte les membres, l'API les stocke dans une table `guild_members`.

### Base de donnees — Nouvelle table

```sql
CREATE TABLE IF NOT EXISTS guild_members (
    guild_id        TEXT NOT NULL,
    user_id         TEXT NOT NULL,
    username        TEXT NOT NULL,
    display_name    TEXT,
    avatar          TEXT,
    roles           JSONB DEFAULT '[]',
    joined_at       TIMESTAMPTZ,
    account_created TIMESTAMPTZ,
    is_bot          BOOLEAN DEFAULT FALSE,
    last_seen_at    TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (guild_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_guild_members_guild ON guild_members (guild_id);
CREATE INDEX IF NOT EXISTS idx_guild_members_username ON guild_members (guild_id, username);
```

### API — Endpoint summary

Le endpoint `/summary` agrege les donnees de plusieurs tables :

```json
{
  "member": {
    "user_id": "288061265727848449",
    "username": "darkponey",
    "display_name": "DarkPoney",
    "avatar": "...",
    "roles": ["Moderateur", "Membre"],
    "joined_at": "2024-01-15T...",
    "account_created": "2020-03-10T..."
  },
  "conduct": {
    "points": 7,
    "max_points": 12,
    "log": [...]
  },
  "infractions": {
    "total": 3,
    "recent": [...]
  },
  "moderation": {
    "total_warns": 1,
    "total_mutes": 0,
    "total_bans": 0,
    "actions": [...]
  },
  "stats": {
    "message_count": 1523,
    "voice_seconds": 36000,
    "last_active": "2026-03-26T..."
  }
}
```

### Desktop App

**Nouvelle page** : `MembersPage.vue`
- Composable `useMembers()` : liste, recherche, tri, filtres
- Composable `useMemberDetail()` : profil complet + summary
- Route `/members`
- Icone sidebar : "users"

**Composants** :
- `MemberCard` — carte resumee d'un membre (avatar, pseudo, points, infractions)
- `MemberProfile` — profil complet avec onglets (Infos, Infractions, Moderation, Stats)
- `MemberActions` — boutons d'action (warn, mute, ban, ajuster points)

### Bots

**security-bot** (ou nouveau members-bot) :
- Au demarrage : scan `guild.members()` → POST /api/members/sync
- Sur `guild_member_addition` : POST /api/members/register
- Sur `guild_member_removal` : DELETE /api/members/{guild_id}/{user_id}
- Sur `guild_member_update` : PATCH /api/members/{guild_id}/{user_id}

---

## Maquette de la page

```
┌─────────────────────────────────────────────────────────┐
│ Membres                                                  │
│ Liste des membres du serveur                             │
│                                                          │
│ [Rechercher un membre...] [Filtre role ▾] [Tri ▾]       │
│                                                          │
│ ┌──────┬────────────┬───────────┬────────┬────────────┐ │
│ │Avatar│ Pseudo      │ Roles     │ Points │ Infractions│ │
│ ├──────┼────────────┼───────────┼────────┼────────────┤ │
│ │ 🟢   │ darkponey  │ Membre    │ 7/12 🟠│ 3          │ │
│ │ 🔴   │ bibi_7777  │ Membre    │ 11/12 🟢│ 1         │ │
│ │ 🟢   │ admin42    │ Admin     │ 12/12 🟢│ 0         │ │
│ └──────┴────────────┴───────────┴────────┴────────────┘ │
│                                                          │
│ ─── Clic sur darkponey ────────────────────────────────  │
│                                                          │
│ ← Retour                                                │
│                                                          │
│ [Avatar] darkponey                                       │
│ ID: 288061265727848449                                   │
│ Membre depuis: 15 jan. 2024                              │
│ Compte cree: 10 mar. 2020                                │
│ Roles: Membre                                            │
│                                                          │
│ Points de conduite: [███████░░░░░] 7/12                  │
│                                                          │
│ [Onglet: Infractions] [Moderation] [Stats]               │
│                                                          │
│ Infractions recentes:                                    │
│ - 26/03 22:32 | Insulte | Score 5.0 | Delete | -2 pts   │
│ - 26/03 22:25 | Insulte | Score 5.0 | Delete | -2 pts   │
│ - 26/03 20:47 | Spam    | Score 3.0 | Warn   | -1 pt    │
│                                                          │
│ [⚠️ Avertir] [🔇 Mute] [🔨 Bannir] [±Points]            │
└─────────────────────────────────────────────────────────┘
```

---

## Plan d'implementation

### Phase 1 — API
1. Migration SQL : table `guild_members`
2. Nouveaux endpoints : list, get, summary, sync, register, remove
3. Modifier security-bot pour sync les membres

### Phase 2 — Desktop App
1. Types TypeScript
2. Composables useMembers + useMemberDetail
3. Page MembersPage.vue avec liste + fiche detail
4. Actions : warn, mute, ban, ajuster points
5. Route + sidebar

### Phase 3 — Temps reel
1. WebSocket events : member_join, member_leave, member_update
2. Mise a jour automatique de la liste dans l'app

---

## Estimation

- API : 1 migration + 3 endpoints + 1 endpoint summary (agregation)
- Bot : modification security-bot pour sync membres
- Desktop : 1 page + 2 composables + route + sidebar
- Total : ~15 fichiers a creer/modifier
