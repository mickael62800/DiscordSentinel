# Ticket Bot

Bot de gestion de tickets pour Discord. Il permet aux utilisateurs de creer des tickets de support, signaler des problemes, faire appel de sanctions, et aux moderateurs de gerer ces tickets.

Conformement a la philosophie du projet, le bot est une **interface legere** : il cree les salons Discord et transmet tout au backend API qui persiste et gere les donnees.

---

## Stack technique

| Composant | Technologie |
|-----------|-------------|
| Langage | Rust (edition 2021) |
| Framework Discord | Serenity 0.12 |
| Runtime async | Tokio |
| Client HTTP | Reqwest 0.12 (rustls) |
| Serialisation | Serde / serde_json |
| Configuration | dotenvy (.env) |
| Logging | tracing + tracing-subscriber |
| Crate partage | sentinel-shared (api_client, config, heartbeat) |

---

## Structure du projet

```
bots/ticket-bot/
├── Cargo.toml
├── Dockerfile
├── .dockerignore
├── .env.example
└── src/
    ├── main.rs              # Point d'entree, init client Discord
    ├── config.rs            # Chargement config depuis .env (extends BaseConfig)
    ├── api_client.rs        # Client HTTP specifique tickets (wraps BaseApiClient)
    ├── handler.rs           # EventHandler : panel interactif + sync messages
    └── commands/
        ├── mod.rs           # Enregistrement des slash commands
        └── ticket.rs        # Panel interactif, creation, fermeture, assignation
```

---

## Fonctionnement actuel

### Panel interactif (methode principale)

Le bot deploie un **panel interactif** dans un salon avec un bouton "Creer un ticket". Quand un utilisateur clique :

1. Un **menu deroulant** apparait avec 6 types de ticket :
   - Probleme serveur
   - Signalement utilisateur
   - Appel de sanction
   - Question / aide
   - Suggestion
   - Autre

2. L'utilisateur selectionne un type
3. Le bot cree un **salon textuel prive** (pas un thread) avec permissions :
   - `@everyone` : DENY (lecture/ecriture)
   - Auteur du ticket : ALLOW (lecture/ecriture)
   - Staff/moderateurs : ALLOW (lecture/ecriture)
4. Le bot envoie `POST /api/tickets` au backend
5. Un message de bienvenue est affiche dans le salon avec les boutons :
   - "Passer en vocal" (cree un salon vocal lie)
   - "Inviter quelqu'un" (ajoute un utilisateur au ticket)
   - "Fermer le ticket"

### Slash commands

#### `/ticket setup`

Deploie le panel interactif dans le salon actuel (admin uniquement).

#### `/ticket close`

Ferme le ticket du salon actuel (doit etre utilise dans un salon `ticket-*`).

1. Le bot envoie `PATCH /api/tickets/{id}/close` au backend
2. Le backend passe le statut a `closed`
3. Le salon Discord est supprime apres 5 secondes

### Sync automatique des messages

Tout message envoye dans un salon `ticket-*` est automatiquement transmis au backend via `POST /api/tickets/{id}/messages`. Le bot determine le role de l'auteur (moderateur ou utilisateur) via les permissions Discord.

Les messages de bots sont ignores pour eviter les boucles.

---

## Ce qui fonctionne (etat actuel)

- [x] Panel interactif avec bouton + menu deroulant (6 types)
- [x] Creation de salon prive avec permissions
- [x] Envoi au backend (`POST /api/tickets`)
- [x] Message de bienvenue avec boutons d'action
- [x] Sync des messages vers le backend
- [x] Fermeture via bouton ou slash command
- [x] Suppression du salon Discord a la fermeture
- [x] Heartbeat vers le backend (via sentinel-shared)
- [x] Enregistrement des guilds au demarrage
- [x] Logging des actions

---

## Ce qui ne fonctionne PAS / bugs connus

### BUG CRITIQUE : Fermeture utilise le nom du salon au lieu de l'UUID

**Fichier :** `commands/ticket.rs` ligne ~334

Le bot extrait l'ID du ticket depuis le nom du salon (`ticket-username-1234`) et l'envoie a l'API. Mais l'API attend un **UUID valide**. Le `close_ticket()` echoue silencieusement car `"username-1234"` n'est pas un UUID.

**Correction :** Stocker l'UUID du ticket (retourne par le backend a la creation) dans le message de bienvenue ou dans les metadata du salon, et l'utiliser pour la fermeture.

### BUG CRITIQUE : voice_channel_id et invited_user_id jamais transmis

**Fichiers :** `api_client.rs` structs Ticket

Les boutons "Passer en vocal" et "Inviter quelqu'un" creent bien les salons/permissions dans Discord, mais les IDs ne sont jamais envoyes au backend (`voice_channel_id`, `invited_user_id`). Ces champs existent en base de donnees mais restent toujours NULL.

**Correction :** Apres creation du salon vocal ou invitation, appeler `PATCH /api/tickets/{id}/channels` (endpoint a creer cote API).

### BUG CRITIQUE : Reponses du staff desktop jamais affichees dans Discord

Quand un moderateur repond a un ticket depuis l'application bureau, le message est sauvegarde en base mais **jamais affiche dans le salon Discord**. La communication est a sens unique (Discord -> backend, mais pas backend -> Discord).

**Correction :** Implementer un listener WebSocket dans le bot qui ecoute les events `ticket_message` et affiche les nouveaux messages dans le salon Discord correspondant.

### BUG : Endpoint update_ticket_channel n'existe pas

La fonction `update_ticket_channel()` est definie dans le service backend et les ports, mais **aucune route HTTP** n'est enregistree dans le router. L'endpoint est mort.

**Correction :** Ajouter la route `PATCH /api/tickets/{id}/channels` dans le router de l'API.

### BUG MINEUR : ticket_type manquant dans les structs bot et desktop

Le champ `ticket_type` (probleme_serveur, signalement, appel, etc.) est envoye a la creation mais n'est pas present dans les structs de deserialisation du bot ni dans les types TypeScript du desktop.

---

## Limitations connues

### Pas de validation des statuts et priorites

Les statuts (`open`, `pending`, `closed`) et priorites (`urgent`, `high`, `medium`, `low`) sont des strings libres. L'API accepte n'importe quelle valeur sans validation. Devrait etre des enums.

### Pas de rate limiting

Un utilisateur peut creer un nombre illimite de tickets. Il faudrait limiter a ~5 tickets par jour par utilisateur.

### Pas de recherche par texte

Ni le bot ni le desktop ne permettent de chercher un ticket par son contenu ou son titre. Le filtrage se fait uniquement par statut/priorite.

### Pas d'historique des assignations

Quand un ticket est reassigne, l'ancien assignataire est ecrase. Aucune trace de qui a assigne a qui et quand.

### Pas de notification lors de nouvelles reponses

Quand un message est ajoute a un ticket (cote Discord ou cote desktop), les autres participants ne recoivent pas de notification push.

---

## Communication avec le backend

| Action | Methode | Endpoint | Etat |
|--------|---------|----------|------|
| Creer un ticket | `POST` | `/api/tickets` | Fonctionne |
| Voir un ticket | `GET` | `/api/tickets/{id}` | Fonctionne |
| Repondre | `POST` | `/api/tickets/{id}/messages` | Fonctionne |
| Fermer | `PATCH` | `/api/tickets/{id}/close` | Bug (UUID vs nom salon) |
| Assigner | `PATCH` | `/api/tickets/{id}/assign` | Fonctionne |
| Mettre a jour channels | `PATCH` | `/api/tickets/{id}/channels` | Route manquante |

L'authentification se fait via le header `Authorization: Bearer <API_KEY>`.

### Requete de creation

```json
{
  "title": "Probleme serveur",
  "priority": "medium",
  "author_id": "111222333",
  "author_name": "pseudo",
  "server": "Mon Serveur",
  "category": "report",
  "ticket_type": "probleme_serveur",
  "channel_id": "999888777"
}
```

### Reponse de creation

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "title": "Probleme serveur",
  "status": "open",
  "priority": "medium",
  "author_id": "111222333",
  "author_name": "pseudo",
  "assigned_to": null,
  "server": "Mon Serveur",
  "category": "report",
  "ticket_type": "probleme_serveur",
  "channel_id": "999888777",
  "voice_channel_id": null,
  "invited_user_id": null,
  "created_at": "2026-03-26T09:00:00+00:00",
  "updated_at": "2026-03-26T09:00:00+00:00",
  "messages_count": 0
}
```

---

## Flux complet

```
Utilisateur clique "Creer un ticket" sur le panel
       |
       v
Menu deroulant : choisit "Signalement utilisateur"
       |
       v
+--------------------------------------+
|           TICKET BOT                 |
|                                      |
|  1. Cree salon prive #ticket-xxx    |
|  2. POST /api/tickets -> Backend     |
+--------------------------------------+
       |
       v
+--------------------------------------+
|           API BACKEND                |
|                                      |
|  1. Cree le ticket en base (PG)     |
|  2. Broadcast event ticket_new (WS) |
|  3. Retourne le ticket avec son ID  |
+--------------------------------------+
       |
       v
+--------------------------------------+
|           TICKET BOT                 |
|                                      |
|  3. Affiche message bienvenue       |
|     + boutons (vocal, inviter,      |
|       fermer)                        |
|  4. Reponse ephemere a l'utilisateur|
+--------------------------------------+
       |
       v
  L'utilisateur et les modos discutent dans le salon
  -> Chaque message est sync vers POST /api/tickets/{id}/messages
       |
       v
  [ Application bureau : le staff voit le ticket et peut repondre ]
  -> PROBLEME : les reponses du staff ne remontent PAS vers Discord
       |
       v
  Un modo clique "Fermer le ticket"
       |
       v
  Bot -> PATCH /api/tickets/{id}/close -> Backend
  Bot -> Supprime le salon Discord apres 5s
```

---

## Configuration

### Variables d'environnement

| Variable | Obligatoire | Description | Defaut |
|----------|-------------|-------------|--------|
| `TICKET_DISCORD_TOKEN` | Oui | Token du bot Discord | - |
| `API_BASE_URL` | Non | URL du backend | `http://localhost:3000` |
| `API_KEY` | Non | Cle API pour l'authentification | _(vide)_ |
| `TICKET_CATEGORY_ID` | Non | ID de categorie Discord pour les salons | _(vide)_ |

### Intents Discord requis

- **GUILD_MESSAGES** : recevoir les messages dans les salons
- **MESSAGE_CONTENT** : lire le contenu des messages pour la sync
- **GUILDS** : acceder aux infos des serveurs

### Permissions Discord requises

| Permission | Raison |
|------------|--------|
| Send Messages | Envoyer des messages dans les salons |
| Manage Channels | Creer/supprimer les salons de tickets |
| Manage Roles | Gerer les permissions des salons prives |
| Send Messages in Threads | Ecrire dans les threads |
| Use Slash Commands | Enregistrer et utiliser les commandes |
| Connect | Creer des salons vocaux lies aux tickets |

---

## Plan de corrections (priorite)

### Phase 1 : Bugs critiques

1. **Fixer close_ticket()** : stocker l'UUID du backend dans les metadata du salon et l'utiliser au lieu du nom
2. **Ajouter route PATCH /api/tickets/{id}/channels** dans le router API
3. **Transmettre voice_channel_id et invited_user_id** au backend apres creation
4. **Ajouter ticket_type** dans les structs de deserialisation (bot + desktop)

### Phase 2 : Communication bidirectionnelle

5. **Afficher les reponses staff dans Discord** : le bot ecoute les events WebSocket `ticket_message` et poste dans le salon
6. **Notifications** : notifier les participants quand un message est ajoute

### Phase 3 : Ameliorations

7. **Enums** pour statuts et priorites (validation compile-time)
8. **Rate limiting** : max 5 tickets/jour/utilisateur
9. **Recherche** : endpoint de recherche par titre/contenu
10. **Historique assignations** : table `ticket_assignments`
