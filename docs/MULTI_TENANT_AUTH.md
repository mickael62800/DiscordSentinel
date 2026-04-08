# Securisation multi-serveur — Isolation des donnees par guild

## Probleme

Actuellement, toute personne ayant l'`API_KEY` peut voir et gerer **tous** les serveurs Discord.
Si tu partages l'application bureau avec un admin qui ne gere qu'un seul serveur, il verra les donnees de tous les autres.

## Solutions possibles

### Solution 1 : Filtrage par guilds via Discord OAuth2 (recommandee)

L'app desktop a deja le login Discord OAuth2 (`auth_service.rs`).
On peut utiliser le token Discord de l'utilisateur pour recuperer ses guilds et filtrer cote API.

**Fonctionnement :**
1. L'utilisateur se connecte via Discord OAuth2 (deja en place)
2. L'API recupere ses guilds via `https://discord.com/api/users/@me/guilds`
3. Chaque requete API est filtree : l'utilisateur ne voit que les guilds dont il est membre avec permission `ADMINISTRATOR`

**Avantages :**
- Zero configuration supplementaire
- Securise par Discord directement
- L'utilisateur voit automatiquement ses guilds

**Inconvenients :**
- Necessite un appel Discord API a chaque requete (cacheable 5 min)
- Depend de la disponibilite de Discord

**Implementation :**
- Nouveau middleware API : `guild_auth_middleware`
- Verifie le token Discord de l'utilisateur
- Cache les guilds autorisees dans Redis (TTL 5 min)
- Refuse l'acces si le guild_id demande n'est pas dans la liste

---

### Solution 2 : API keys par guild

Generer une API key unique par guild. Chaque cle ne donne acces qu'aux donnees de son guild.

**Fonctionnement :**
1. L'admin genere une cle pour chaque guild via l'interface
2. L'app desktop stocke la cle
3. L'API verifie que la cle correspond au guild_id de la requete

**Avantages :**
- Simple a implementer
- Pas de dependance Discord
- Fonctionne offline

**Inconvenients :**
- Gestion manuelle des cles
- Pas de granularite (admin vs moderateur)
- Si la cle fuite, acces complet au guild

---

### Solution 3 : RBAC (Role-Based Access Control)

Systeme complet avec utilisateurs, roles et permissions.

**Fonctionnement :**
1. Table `api_users` avec identifiant Discord
2. Table `api_user_guilds` (user_id, guild_id, role)
3. Roles : `owner`, `admin`, `moderator`, `viewer`
4. Chaque endpoint verifie le role requis

**Avantages :**
- Granularite fine (viewer ne peut pas ban, moderator ne peut pas configurer)
- Audit trail complet
- Scalable

**Inconvenients :**
- Plus complexe a implementer
- Interface de gestion des permissions necessaire
- Overhead sur chaque requete

---

## Recommandation

**Phase 1 (rapide)** : Solution 1 — OAuth2 Discord
- L'utilisateur se connecte, l'API filtre automatiquement par ses guilds
- Pas de configuration, ca marche tout seul

**Phase 2 (plus tard)** : Solution 3 — RBAC
- Ajouter des roles quand le besoin de granularite se fait sentir
- Permet de donner un acces "viewer" a quelqu'un sans lui donner les droits admin

## Fichiers a modifier (Solution 1)

```
services/api/src/adapters/inbound/http/middleware/guild_auth.rs  (nouveau)
services/api/src/adapters/inbound/http/router.rs                (ajouter le middleware)
services/api/src/adapters/inbound/http/state.rs                 (cache guilds Redis)
apps/desktop/src-tauri/src/infrastructure/api_adapter.rs        (envoyer le token Discord)
```
