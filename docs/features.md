# Features - Roadmap & Modifications planifiees

## 1. Refonte du ticket-bot : Panneau d'assistance

### Probleme actuel
Le ticket-bot fonctionne via slash commands (`/ticket create`, `/ticket close`, `/ticket assign`). Ce n'est pas assez accessible, surtout pour des situations graves.

### Nouveau systeme
Un **panneau d'assistance** dans un salon textuel dedie avec un menu deroulant (select menu Discord).

#### Categories de ticket
| Categorie | Gravite | Visibilite |
|---|---|---|
| Question simple | 0 | Modos + Admins |
| Probleme technique | 1 | Modos + Admins |
| Signalement d'un membre | 2 | Modos + Admins |
| Harcelement | 3 | Modos + Admins |
| Situation urgente / grave | 4 | Modos + Admins |
| **Probleme avec un moderateur** | 5 | **Admins uniquement** |

#### Fonctionnement
1. L'utilisateur se rend dans le salon d'assistance
2. Un embed permanent affiche un select menu avec les categories
3. L'utilisateur choisit la categorie et valide
4. Un **thread prive** est cree, visible par :
   - L'auteur du ticket
   - Les **admins** (toujours)
   - Les **moderateurs** (sauf si categorie = "probleme avec un moderateur")
5. Discussion dans le thread entre l'utilisateur et le staff
6. Possibilite d'**ajouter un temoin** ou une autre personne a la conversation
7. Fermeture du ticket par un admin/moderateur

#### Parametres necessaires (par serveur)
- ID du salon d'assistance
- ID du role Admin
- ID du role Moderateur

---

## 2. Configuration multi-serveurs des bots

### Probleme actuel
Les bots utilisent des variables d'environnement pour leur configuration (IDs de channels, roles, etc.). Cela pose probleme quand un bot est present sur **plusieurs serveurs** car les IDs sont differents par serveur.

### Solution : Configuration par guild en base de donnees

#### Principe
Remplacer les variables d'environnement specifiques a un serveur par une **table de configuration par guild** en base de donnees. Les parametres globaux (token Discord, URL API) restent en env vars.

#### Table `bot_guild_config`
```sql
CREATE TABLE bot_guild_config (
    id UUID PRIMARY KEY,
    guild_id VARCHAR(20) NOT NULL,
    bot_name VARCHAR(50) NOT NULL,
    config_key VARCHAR(100) NOT NULL,
    config_value TEXT NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(guild_id, bot_name, config_key)
);
```

#### Exemples de config par guild
| guild_id | bot_name | config_key | config_value |
|---|---|---|---|
| 1486472... | voice-bot | public_creator_channel_id | 1486479... |
| 1486472... | voice-bot | private_creator_channel_id | 1486480... |
| 1486472... | voice-bot | log_channel_id | 1486496... |
| 1486472... | ticket-bot | assistance_channel_id | 1486500... |
| 1486472... | ticket-bot | admin_role_id | 1486501... |
| 1486472... | ticket-bot | moderator_role_id | 1486502... |

#### Deux modes de configuration
1. **Via l'application desktop** : interface pour gerer les parametres de chaque bot par serveur
2. **Via Discord** : slash command `/config` pour les admins (ex: `/config set ticket-bot assistance_channel #assistance`)

#### Endpoints API necessaires
- `GET /api/config/{guild_id}/{bot_name}` — recuperer la config d'un bot pour un serveur
- `POST /api/config` — sauvegarder une config
- `DELETE /api/config/{guild_id}/{bot_name}/{key}` — supprimer une config

#### Impact sur les bots
- Au demarrage, les bots chargent leur config depuis l'API au lieu des env vars
- A chaque event Discord (guild join, voice state, message), le bot recupere la config du guild concerne (avec cache Redis)
- Les env vars restent pour : `DISCORD_TOKEN`, `API_BASE_URL`, `API_KEY`

#### Impact sur l'app desktop
- Nouvelle page "Configuration des bots" avec :
  - Selection du serveur
  - Selection du bot
  - Formulaire de config avec les cles disponibles
  - Sauvegarde via l'API

---

## 3. Historique des salons vocaux (soft-delete)

### Statut : IMPLEMENTE

Les salons vocaux temporaires ne sont plus supprimes de la base de donnees. Ils passent en `channel_status = 'closed'` avec une date `closed_at`. L'app desktop n'affiche que les salons `open`, mais l'historique complet est conserve.
