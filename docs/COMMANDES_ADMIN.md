# 🛡️ Commandes Admin — Sentinel Bot

> Toutes les commandes réservées aux modérateurs et administrateurs, regroupées par section. À copier sur le salon staff Discord.

---

# 🛡️ Modération directe

## `/warn`
> Avertir un utilisateur.
- **Permission** : `Modérer les membres`
- **Options** : `user` (membre) · `gravity` (Faible / Moyenne / Haute) · `reason` (raison)
- **Effet** : enregistre un avertissement gradué. Peut déclencher une escalade auto (mute/ban) selon la config. L'utilisateur reçoit un DM.

## `/unwarn`
> Retirer un avertissement.
- **Permission** : `Modérer les membres`
- **Options** : `user` (membre) · `all` *(optionnel)* — supprime TOUS les warns + reset les strikes
- **Effet** : affiche les warns du user avec boutons de suppression individuelle.

## `/mute`
> Mute un utilisateur (permanent ou temporaire).
- **Permission** : `Modérer les membres`
- **Options** : `user` · `reason` · `duration` *(optionnel, en minutes — vide = permanent, max 28 jours)*
- **Effet** : applique le timeout Discord. Confirmation demandée si compte à risque. DM envoyé.

## `/unmute`
> Retirer le mute d'un utilisateur.
- **Permission** : `Modérer les membres`
- **Options** : `user` (membre)
- **Effet** : retire immédiatement le timeout et log l'action.

## `/ban`
> Bannir un utilisateur (permanent ou temporaire).
- **Permission** : `Bannir des membres`
- **Options** : `user` · `reason` · `duration` *(optionnel, en heures — vide = permanent)*
- **Effet** : bannit le membre. Confirmation demandée si profil à risque. DM de ban envoyé.

## `/unban`
> Débannir un utilisateur.
- **Permission** : `Bannir des membres`
- **Options** : `user_id` (ID Discord du membre)
- **Effet** : retire le ban et enregistre l'action.

## `/massmute`
> Mute en masse jusqu'à 200 membres.
- **Permission** : `Modérer les membres`
- **Options** : `users` (IDs séparés par espaces ou virgules) · `reason` · `duration` *(défaut 10 min)*
- **Effet** : mute temporaire de plusieurs membres d'un coup. Récap reçus / échecs / immunisés.

## `/massban`
> Ban en masse jusqu'à 200 membres.
- **Permission** : `Bannir des membres`
- **Options** : `users` (IDs séparés par espaces ou virgules) · `reason`
- **Effet** : bannit plusieurs comptes en une commande. Récap affiché.

---

# 📋 Investigation & historique

## `/history`
> Historique des sanctions d'un utilisateur.
- **Permission** : `Modérer les membres`
- **Options** : `user`
- **Effet** : affiche les 10 dernières sanctions (warns / mutes / bans) avec totaux par type.

## `/note`
> Ajouter une note interne sur un utilisateur.
- **Permission** : `Modérer les membres`
- **Options** : `user` · `content` · `category` *(General / Avertissement / Positif / Contexte)*
- **Effet** : note persistante visible des modos pour suivi long-terme.

## `/evidence`
> Joindre / lister des preuves sur une action de modération.
- **Permission** : `Modérer les membres`
- **Sous-commandes** :
  - `add action_id url [description]` — joint une URL à une action existante
  - `list action_id` — liste les preuves attachées
- **Effet** : centralise les screenshots et liens de preuve par action.

## `/compare`
> Comparer l'historique de 2 utilisateurs.
- **Permission** : `Modérer les membres`
- **Options** : `user1` · `user2`
- **Effet** : tableau comparatif warns / mutes / bans entre 2 membres. Indique qui a le plus de sanctions.

## `/context`
> Voir les messages autour d'un message précis.
- **Permission** : `Modérer les membres`
- **Options** : `message_id` · `count` *(défaut 5)*
- **Effet** : affiche le message cible + N avant et après. Utile pour les enquêtes.

## `/expirations`
> Liste les sanctions temporaires actives.
- **Permission** : `Modérer les membres`
- **Effet** : tous les mutes / bans temporaires en cours, triés par expiration la plus proche.

## `/export`
> Exporter l'historique de modération d'un utilisateur.
- **Permission** : `Modérer les membres`
- **Options** : `user` · `format` *(JSON / CSV)*
- **Effet** : génère un fichier téléchargeable contenant toutes les sanctions du user.

## `/review`
> File de relecture (seconde opinion).
- **Permission** : `Modérer les membres` *(resolve : senior mods)*
- **Sous-commandes** :
  - `add action_id [reason]` — demander une review
  - `list` — voir les reviews en attente
  - `resolve review_id status [notes]` — valider (Approved / Rejected / Changed)
- **Effet** : workflow de validation à deux niveaux pour les actions sensibles.

## `/template`
> Gérer les templates de raisons de modération.
- **Permission** : `Administrateur`
- **Sous-commandes** :
  - `list` — affiche les templates existants
  - `add label reason` — crée un nouveau template
  - `remove label` — supprime un template
- **Effet** : raccourcis personnalisés pour accélérer les modérations et garder de la cohérence.

## `/transcript`
> Transcript des 100 derniers messages d'un salon.
- **Permission** : `Modérer les membres`
- **Options** : `channel`
- **Effet** : télécharge un fichier `.txt` avec timestamps et auteurs. Pour archives ou enquêtes.

## `/call`
> Convoquer un membre dans un salon privé.
- **Permission** : `Modérer les membres`
- **Options** : `user` · `reason` *(optionnel)*
- **Effet** : crée un salon privé entre le modo et le membre. Bouton de fermeture (suppression auto 3s après).

## `/modstats`
> Métriques d'activité des modérateurs (30 derniers jours).
- **Permission** : `Modérer les membres`
- **Effet** : classement des modos par nombre d'actions (warns / mutes / bans / kicks). Médailles top 3.

## `/audit`
> Commandes du module audit.
- **Permission** : `Modérer les membres`
- **Sous-commandes** :
  - `search user [type] [limit]` — recherche dans les logs d'audit
  - `stats` — statistiques hebdomadaires (joins, départs, bans, suppressions, edits, anomalies)
- **Effet** : investigation fine sur les événements serveur.

---

# 🤖 Automod & sécurité

## `/automod`
> État et test de l'automod.
- **Permission** : `Gérer le serveur`
- **Sous-commandes** :
  - `status` — état actuel (caches, trackers)
  - `test message` — analyse un message en mode dry-run (spam / insulte / liens / phishing)
- **Effet** : check de santé + outil de test sans action.

## `/security`
> État et historique du module sécurité.
- **Permission** : `Gérer le serveur`
- **Sous-commandes** :
  - `status` — joins récents, lockdown, quarantaine
  - `history [limit]` — derniers événements (raids, patterns suspects)
- **Effet** : visibilité sur l'activité anti-raid.

---

# 🧹 Nettoyage

## `/cleanup`
> Purge des données anciennes (admin).
- **Permission** : `Administrateur`
- **Sous-commandes** :
  - `logs jours` — purge les logs système plus vieux que N jours
  - `infractions jours` — purge les infractions
  - `audit jours` — purge les logs d'audit
- **Effet** : réduit la taille de la base de données. Bornes 1-365 jours.

## `/purge`
> Suppression de messages dans le salon courant.
- **Permission** : `Gérer les messages`
- **Sous-commandes** :
  - `last nombre` — N derniers messages
  - `user utilisateur nombre` — messages d'un user dans les N derniers
  - `contains texte nombre` — messages contenant un texte
  - `bots nombre` — messages de bots
  - `links nombre` — messages avec liens
  - `attachments nombre` — messages avec fichiers
  - `all confirmation` — TOUS les messages (taper `CONFIRMER`, irréversible)
- **Effet** : suppression en masse avec rate-limit auto. Cap à 100 par sous-commande.

---

# ⚙️ Configuration jeux

## `/blackjack-setup`
> Déployer le panneau Blackjack dans ce salon.
- **Permission** : `Gérer le serveur`
- **Effet** : message permanent avec bouton pour ouvrir une table privée (mise + jeu).

## `/slot-setup`
> Déployer le panel Machine à sous dans ce salon.
- **Permission** : `Gérer le serveur`
- **Effet** : message permanent avec bouton pour ouvrir la machine à sous privée (spins + daily bonus).

## `/wheel-setup`
> Déployer le panel Roue du Destin dans ce salon.
- **Permission** : `Gérer le serveur`
- **Effet** : message permanent avec bouton pour tourner la Roue. Résultat affiché publiquement.

## `/taunts-channel`
> Configurer le salon des railleries automatiques (Coup de Coude).
- **Permission** : `Gérer le serveur`
- **Options** : `salon` *(optionnel — omettre = désactiver)*
- **Effet** : définit où les taunts et changements de pseudo sont postés. Le bot a besoin de **Gérer les pseudos** pour les renames.

---

# 🎮 Configuration serveur

## `/game-admin`
> Gérer les jeux Discord (rôles auto).
- **Permission** : `Gérer le serveur`
- **Sous-commandes** :
  - `create name [emoji] [category]` — crée un jeu (génère le rôle Discord)
  - `delete name` — supprime un jeu
  - `panel [category]` — déploie un panel de sélection (select menu)
  - `refresh [category]` — recharge un panel existant
- **Effet** : système de tag-jeux pour que les membres s'inscrivent (LFG, RPG, FPS…).

## `/ticket-admin`
> Administration du système de tickets.
- **Permission** : `Gérer les salons`
- **Sous-commandes** :
  - `panel` — déploie le panneau de création de ticket dans ce salon
  - `invite membre` — ajoute un membre à un ticket existant
- **Effet** : panneau permanent pour ouvrir un ticket support, et invitation manuelle de membres.

## `/roles-panel`
> Gérer les panels de rôles auto-réaction.
- **Permission** : `Gérer le serveur`
- **Sous-commandes** :
  - `deploy panel_id` — déploie un panel (ID depuis le dashboard desktop)
  - `list` — liste les panels du serveur et leur état
- **Effet** : panel auto-rôle géré côté web admin, déployé via cette commande.

---

# 📈 Progression

## `/progression-resync`
> Force la vérification des rôles de niveau (texte / vocal / jours).
- **Permission** : `Gérer le serveur`
- **Sous-commandes** :
  - `user @cible` — re-vérifie un membre précis
  - `me` — re-vérifie soi-même
  - `all [limit]` — top N joueurs *(défaut 50, max 200, throttle 250 ms)*
- **Effet** : réapplique les rôles XP en lisant l'état actuel. Utile si nouveau reward ajouté à posteriori, changement de mode `xp_role_mode`, ou attribution Discord ratée historiquement.

---

# 📌 Récap des permissions

| Permission Discord | Commandes |
|---|---|
| **Administrateur** | `/cleanup`, `/template` |
| **Bannir des membres** | `/ban`, `/unban`, `/massban` |
| **Gérer le serveur** | `/automod`, `/security`, `/blackjack-setup`, `/slot-setup`, `/wheel-setup`, `/taunts-channel`, `/game-admin`, `/roles-panel`, `/progression-resync` |
| **Gérer les salons** | `/ticket-admin` |
| **Gérer les messages** | `/purge` |
| **Modérer les membres** | `/warn`, `/unwarn`, `/mute`, `/unmute`, `/massmute`, `/history`, `/note`, `/evidence`, `/compare`, `/context`, `/expirations`, `/export`, `/review`, `/transcript`, `/call`, `/modstats`, `/audit` |

---

*Document à jour au 2026-04-26. Pour les commandes joueur (gameplay Coup de Coude), voir `COUP_DE_COUDE_COMMANDES.md`.*
