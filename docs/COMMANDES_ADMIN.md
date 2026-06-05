# 🛡️ Commandes Modérateurs & Administrateurs — Bot Sentinelle

> Commandes nécessitant une permission Discord (Admin, Gérer le serveur, Modérer les membres,
> Gérer les messages, Gérer les salons). Réservées au staff.
>
> Voir aussi : [COMMANDES_UTILISATEURS.md](./COMMANDES_UTILISATEURS.md) pour les commandes publiques.
> Source : `sentinel-bot/src/modules/`.

## Sommaire

| Module | Commandes | Permission |
|---|---|---|
| `cleanup-bot` | `/cleanup`, `/purge` | Administrateur / Gérer les messages |
| `game-bot` | `/game-admin` | Gérer le serveur |
| `community-bot` | `/roles-panel` | Gérer le serveur |
| `audit-bot` | `/audit` | Modérer les membres |
| `progression-bot` | `/progression-resync` | Gérer le serveur |
| `blackjack-bot` | `/blackjack-setup` | Gérer le serveur |
| `slot-bot` | `/slot-setup` | Gérer le serveur |
| `wheel-bot` | `/wheel-setup` | Gérer le serveur |
| `security-bot` | `/security` | Gérer le serveur |
| `automod-bot` | `/automod` | Gérer le serveur |
| `moderation-bot` | 21 commandes | Permissions de modération |
| `coude-bot` | `/taunts-channel` | Admin |
| `ticket-bot` | `/ticket-admin` | Gérer les salons |
| `confessions` | `/confess-admin` | Admin |

---

## 🧹 cleanup-bot — Nettoyage

### `/cleanup` — *Nettoyer les données anciennes (admin)*
Permission requise : **Administrateur**.

| Sous-commande | Description | Arguments |
|---|---|---|
| `logs` | Purger les logs système plus anciens que X jours | `jours` (1-365, requis) |
| `infractions` | Purger les infractions plus anciennes que X jours | `jours` (1-365, requis) |
| `audit` | Purger les logs d'audit plus anciens que X jours | `jours` (1-365, requis) |

### `/purge` — *Supprimer des messages dans le salon*
Permission requise : **Gérer les messages**.

| Sous-commande | Description | Arguments |
|---|---|---|
| `last` | Supprimer les X derniers messages | `nombre` (1-100, requis) |
| `user` | Supprimer les messages d'un utilisateur | `utilisateur` (requis), `nombre` (1-100, requis) |
| `contains` | Supprimer les messages contenant un texte | `texte` (requis) |

---

## 🎮 game-bot — Gestion des jeux

### `/game-admin` — *Gérer les jeux (admin)*
Permission requise : **Gérer le serveur**.

| Sous-commande | Description | Arguments |
|---|---|---|
| `create` | Créer un jeu | `name` (requis), `emoji` (optionnel), `category` (optionnel) |
| `delete` | Supprimer un jeu | `name` (requis) |
| `panel` | Déployer le panneau d'une catégorie | `category` (optionnel) |
| `refresh` | Rafraîchir le panneau d'une catégorie | `category` (optionnel) |

---

## 👥 community-bot — Communauté

### `/roles-panel` — *Gérer les panels de rôles*
Permission requise : **Gérer le serveur**. Déploie un panneau de rôles auto-assignables
(boutons, groupes exclusifs, prérequis personnalisables).

---

## 🔎 audit-bot — Audit

### `/audit` — *Commandes du audit bot*
Permission requise : **Modérer les membres**.

| Sous-commande | Description | Arguments |
|---|---|---|
| `search` | Rechercher dans les logs d'audit | `user` (requis), `type` (optionnel, ex : `message_delete`, `member_ban`), `limit` (1-50, optionnel, défaut 10) |
| `stats` | Affiche les statistiques hebdomadaires | — |

---

## 📈 progression-bot — Niveaux & XP

### `/progression-resync` — *Force la vérification des rôles de niveau (texte/vocal/jours)*
Permission requise : **Gérer le serveur**.

---

## 🃏 blackjack-bot — Casino

### `/blackjack-setup` — *Déployer le panneau de Blackjack dans ce salon (admin)*
Permission requise : **Gérer le serveur**. Pose un bouton « Jouer au Blackjack » qui ouvre
une table privée.

---

## 🎰 slot-bot — Machine à sous

### `/slot-setup` — *Déployer le panel Machine à sous dans ce salon (admin)*
Permission requise : **Gérer le serveur**.

---

## 🌀 wheel-bot — Roue du Destin

### `/wheel-setup` — *Déployer le panel Roue du Destin dans ce salon (admin)*
Permission requise : **Gérer le serveur**.

---

## 🔐 security-bot — Sécurité

### `/security` — *Commandes du security bot*
Permission requise : **Gérer le serveur**.

| Sous-commande | Description | Arguments |
|---|---|---|
| `status` | Affiche l'état actuel de la sécurité | — |
| `history` | Affiche les derniers événements de sécurité | `limit` (1-25, optionnel, défaut 5) |

---

## 🤖 automod-bot — Modération automatique

### `/automod` — *Commandes de l'automod bot*
Permission requise : **Gérer le serveur**.

| Sous-commande | Description | Arguments |
|---|---|---|
| `status` | Affiche l'état actuel de l'automod (caches, trackers) | — |
| `test` | Teste l'analyse d'un message | `message` (requis) |

---

## ⚖️ moderation-bot — Modération manuelle

Réservé aux modérateurs (21 commandes).

**Sanctions :**
| Commande | Ce qu'elle fait |
|---|---|
| `/warn` | Donne un avertissement officiel à un membre (enregistré dans son historique) |
| `/unwarn` | Annule un avertissement précédemment donné |
| `/mute` | Réduit un membre au silence, de façon permanente ou pour une durée donnée |
| `/unmute` | Lève le mute d'un membre avant son expiration |
| `/ban` | Bannit un membre du serveur, de façon permanente ou temporaire |
| `/unban` | Lève le bannissement d'un membre |
| `/massmute` | Mute plusieurs membres d'un coup (utile lors d'un raid) |
| `/massban` | Bannit plusieurs membres en une seule commande |

**Suivi & dossiers :**
| Commande | Ce qu'elle fait |
|---|---|
| `/history` | Affiche tout l'historique de sanctions d'un membre |
| `/note` | Ajoute une note interne (visible du staff) sur un membre, sans le sanctionner |
| `/context` | Affiche les messages autour d'un message précis pour comprendre une situation |
| `/evidence` | Attache des preuves (captures, liens) à une sanction, ou les liste |
| `/expirations` | Liste les sanctions temporaires en cours avec le temps restant |
| `/compare` | Compare côte à côte les historiques de deux membres |
| `/call` | Convoque un membre dans un salon privé pour discuter |

**Outils & supervision :**
| Commande | Ce qu'elle fait |
|---|---|
| `/appeal` | Permet de contester une sanction reçue (ouvre automatiquement un ticket) |
| `/review` | File de relecture : un autre modérateur valide / donne une seconde opinion |
| `/template` | Gère les modèles de raisons de sanction réutilisables (senior mods) |
| `/transcript` | Génère un export texte des 100 derniers messages d'un salon |
| `/export` | Exporte l'historique de modération complet d'un membre |
| `/modstats` | Statistiques d'activité des modérateurs sur les 30 derniers jours |

---

## 🥊 coude-bot — Configuration (admin)

### `/taunts-channel` — *(Admin) Configure le salon des railleries automatiques*

---

## 🎫 ticket-bot — Tickets

### `/ticket-admin` — *Administration des tickets (staff)*
Permission requise : **Gérer les salons**.

| Sous-commande | Description | Arguments |
|---|---|---|
| `panel` | Déployer le panneau de création de ticket dans ce salon | — |
| `invite` | Inviter un membre dans ce ticket | `membre` (requis) |

---

## 🤫 confessions — Administration

### `/confess-admin` — *Administration des confessions (admin only)*

| Sous-commande | Description | Arguments |
|---|---|---|
| `deploy-panel` | Poste le bouton « Poster une confession » dans ce canal | — |
| `delete` | Supprime une confession par numéro | `number` (requis, ex : 350) |
| `reveal` | Révèle l'auteur d'une confession (owner only) | `number` (requis) |
