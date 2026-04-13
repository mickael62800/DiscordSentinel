# Checklist de test — Commandes des bots DiscordSentinel

Liste exhaustive des commandes slash à tester pour valider l'ensemble des bots.
Chaque commande indique le **rôle attendu** et ce que chaque profil doit voir.

## Légende des rôles

- **U** = Utilisateur lambda (aucune permission spéciale)
- **M** = Modérateur (`MODERATE_MEMBERS`, `MANAGE_MESSAGES`)
- **A** = Administrateur (`ADMINISTRATOR` ou `MANAGE_GUILD`)
- ✅ = doit voir / pouvoir exécuter
- ❌ = doit être masquée OU répondre « permission refusée »

> Les commandes restreintes par `default_member_permissions` ne doivent **pas apparaître** dans l'autocomplétion Discord pour les profils non autorisés. Vérifier les deux : visibilité ET exécution.

---

## audit-bot

| Commande | Description | U | M | A |
|---|---|---|---|---|
| `/audit search <user?> <event_type?>` | Recherche dans les logs d'audit | ❌ | ✅ | ✅ |
| `/audit stats` | Statistiques hebdomadaires | ❌ | ✅ | ✅ |

**À vérifier** : les résultats sont paginés, les events sont filtrés par guilde, l'export est lisible.

---

## automod-bot

| Commande | Description | U | M | A |
|---|---|---|---|---|
| `/automod status` | État courant de l'automod | ❌ | ❌ | ✅ |
| `/automod test <message>` | Teste l'analyse d'un message | ❌ | ❌ | ✅ |

**À vérifier** : `test` retourne un score de toxicité cohérent, `status` affiche le seuil et les classes actives.

---

## blackjack-bot

| Commande | Description | U | M | A |
|---|---|---|---|---|
| `/blackjack-setup` | Déploie le panneau Blackjack dans le salon | ❌ | ❌ | ✅ |
| (Boutons du panneau) | Lancer une partie via le panneau | ✅ | ✅ | ✅ |

**À vérifier** :
- Le panneau persiste après redémarrage du bot.
- Une partie ne peut pas être manipulée par un autre joueur (anti-triche boutons).
- Le payout est correctement crédité au profil Coup de Coude.
- Les boutons d'une partie d'un autre joueur sont refusés.

---

## cleanup-bot

| Commande | Description | U | M | A |
|---|---|---|---|---|
| `/cleanup logs <jours>` | Purge logs système | ❌ | ❌ | ✅ |
| `/cleanup infractions <jours>` | Purge infractions | ❌ | ❌ | ✅ |
| `/cleanup audit <jours>` | Purge audit | ❌ | ❌ | ✅ |
| `/purge last <nombre>` | Supprime les N derniers messages | ❌ | ✅ | ✅ |
| `/purge user <user> <nombre>` | Supprime les N derniers messages d'un user | ❌ | ✅ | ✅ |

**À vérifier** : les purges respectent la limite Discord (14 jours) et loguent l'action dans audit-bot.

---

## community-bot

| Commande | Description | U | M | A |
|---|---|---|---|---|
| `/parrain <filleul>` | Parrainer un nouveau membre | ✅ | ✅ | ✅ |

**À vérifier** :
- Le filleul reçoit une demande de confirmation (DM ou message éphémère).
- Le parrainage n'est validé qu'après confirmation.
- Un membre ne peut pas être parrainé deux fois.

---

## coude-bot (Coup de Coude v2)

| Commande | Description | U | M | A |
|---|---|---|---|---|
| `/coude <adversaire>` | Défier un joueur en combat | ✅ | ✅ | ✅ |
| `/profil [user?]` | Affiche le profil | ✅ | ✅ | ✅ |
| `/shop` | Boutique | ✅ | ✅ | ✅ |
| `/prime <cible> <montant>` | Mettre une prime sur un joueur | ✅ | ✅ | ✅ |
| `/leaderboard` | Classement | ✅ | ✅ | ✅ |
| `/pari <combat_id> <joueur> <montant>` | Parier sur un combat | ✅ | ✅ | ✅ |
| `/voler <cible>` | Tenter un vol (interactif) | ✅ | ✅ | ✅ |
| `/assurance` | Acheter une assurance combat | ✅ | ✅ | ✅ |
| `/train <stat>` | Dépenser un point de stat | ✅ | ✅ | ✅ |
| `/classe <classe>` | Choisir/changer de classe | ✅ | ✅ | ✅ |
| `/donner <user> <montant ou item>` | Transférer pièces/items | ✅ | ✅ | ✅ |
| `/hp` | Voir ses HP courants | ✅ | ✅ | ✅ |
| `/repos` | Récupérer tous les HP (cooldown 12h) | ✅ | ✅ | ✅ |
| `/saison` | Infos sur la saison en cours | ✅ | ✅ | ✅ |
| `/reset-stats` | Redistribuer les points (300 pièces) | ✅ | ✅ | ✅ |

**À vérifier (refonte v2)** :
- Combat multi-rounds avec gestion des HP entre rounds.
- Classes cachées révélées au bon moment.
- Surenchère fonctionnelle.
- Vol interactif (boutons cible/voleur).
- Cooldown `/repos` strictement appliqué.
- Les primes sont payées au vainqueur.
- `/pari` rejeté après le début du combat.

---

## game-bot

| Commande | Description | U | M | A |
|---|---|---|---|---|
| `/game create <nom> ...` | Créer un évènement de jeu | ❌ | ❌ | ✅ |
| `/game delete <id>` | Supprimer un évènement | ❌ | ❌ | ✅ |
| `/game list` | Liste des jeux disponibles | ✅ | ✅ | ✅ |
| `/game join <id>` | S'inscrire à un jeu | ✅ | ✅ | ✅ |
| `/game leave <id>` | Se désinscrire | ✅ | ✅ | ✅ |
| `/game my-games` | Mes inscriptions | ✅ | ✅ | ✅ |
| `/game players <id>` | Voir les inscrits | ✅ | ✅ | ✅ |

**À vérifier** : la dédup à l'inscription, le cooldown anti-spam, l'absence d'injection d'URL dans les champs nom/description.

---

## image-bot

| Commande | Description | U | M | A |
|---|---|---|---|---|
| `/image stats` | Statistiques du bot image | ❌ | ❌ | ✅ |

**À vérifier (audit récent)** :
- Aucun panic sur image corrompue.
- Téléchargement avec timeout et taille max appliqués.
- Les images postées par le staff ne sont pas analysées.

---

## moderation-bot

### Sanctions

| Commande | Description | U | M | A |
|---|---|---|---|---|
| `/warn <user> <raison>` | Avertir | ❌ | ✅ | ✅ |
| `/unwarn <user> <id>` | Retirer un warning | ❌ | ✅ | ✅ |
| `/mute <user> <durée?> <raison>` | Mute (perm ou temp) | ❌ | ✅ | ✅ |
| `/unmute <user>` | Démuter | ❌ | ✅ | ✅ |
| `/ban <user> <durée?> <raison>` | Bannir | ❌ | ✅ | ✅ |
| `/unban <user_id>` | Débannir | ❌ | ✅ | ✅ |
| `/massmute <users>` | Mute multiple | ❌ | ✅ | ✅ |
| `/massban <users>` | Ban multiple | ❌ | ✅ | ✅ |

### Consultation & contexte

| Commande | Description | U | M | A |
|---|---|---|---|---|
| `/history <user>` | Historique des sanctions | ❌ | ✅ | ✅ |
| `/note <user> <texte>` | Note interne | ❌ | ✅ | ✅ |
| `/context <message_id>` | Messages autour d'un message | ❌ | ✅ | ✅ |
| `/transcript` | Transcript des 100 derniers messages | ❌ | ✅ | ✅ |
| `/compare <userA> <userB>` | Comparer deux historiques | ❌ | ✅ | ✅ |
| `/export <user>` | Exporter l'historique | ❌ | ✅ | ✅ |
| `/expirations` | Sanctions temporaires actives | ❌ | ✅ | ✅ |
| `/modstats [user?]` | Activité d'un modérateur (30j) | ❌ | ✅ | ✅ |
| `/call <user>` | Convoquer en salon privé | ❌ | ✅ | ✅ |

### Appel utilisateur

| Commande | Description | U | M | A |
|---|---|---|---|---|
| `/appeal <sanction_id> <texte>` | Contester une sanction reçue | ✅ | ✅ | ✅ |

### Preuves & revue

| Commande | Description | U | M | A |
|---|---|---|---|---|
| `/evidence add <action_id> <url>` | Attacher une preuve | ❌ | ✅ | ✅ |
| `/evidence list <action_id>` | Lister les preuves | ❌ | ✅ | ✅ |
| `/review add <action_id>` | Ajouter à la file de revue | ❌ | ✅ | ✅ |
| `/review list` | File de revue en attente | ❌ | ✅ | ✅ |
| `/review resolve <id> <décision>` | Résoudre une revue | ❌ | ✅ | ✅ |

### Templates de raison (admin)

| Commande | Description | U | M | A |
|---|---|---|---|---|
| `/template list` | Lister les templates | ❌ | ❌ | ✅ |
| `/template add <nom> <texte>` | Ajouter un template | ❌ | ❌ | ✅ |
| `/template remove <nom>` | Retirer un template | ❌ | ❌ | ✅ |

**À vérifier** :
- Les sanctions temporaires expirent correctement (vérifier `/expirations` après expiration).
- Un user **ne peut pas** voir les commandes de mod dans l'autocomplétion Discord.
- `/appeal` ne fonctionne **que** pour la sanction du user appelant.
- Les DMs aux sanctionnés partent (sauf DMs fermés).

---

## progression-bot

| Commande | Description | U | M | A |
|---|---|---|---|---|
| `/stats user [user?]` | Stats d'un utilisateur | ✅ | ✅ | ✅ |
| `/stats server` | Stats du serveur | ✅ | ✅ | ✅ |
| `/stats top` | Top des membres actifs | ✅ | ✅ | ✅ |
| `/level user [user?]` | Niveau et XP | ✅ | ✅ | ✅ |
| `/level top` | Classement XP | ✅ | ✅ | ✅ |

**À vérifier** : les XP s'incrémentent à l'envoi de message (avec cooldown anti-spam).

---

## roles-bot

| Commande | Description | U | M | A |
|---|---|---|---|---|
| `/roles-panel deploy <message>` | Déployer un panneau de rôles | ❌ | ❌ | ✅ |
| `/roles-panel list` | Lister les panneaux | ❌ | ❌ | ✅ |
| (Boutons/menu sélection) | Attribuer/retirer un rôle | ✅ | ✅ | ✅ |

**À vérifier** : le panneau survit à un redémarrage, les rôles requis existent, pas d'escalade de privilèges.

---

## security-bot

| Commande | Description | U | M | A |
|---|---|---|---|---|
| `/security status` | État sécurité courant | ❌ | ❌ | ✅ |
| `/security history` | Évènements sécurité récents | ❌ | ❌ | ✅ |

**À vérifier** : les détections (raid, mass-join, lien suspect) sont remontées dans `/security history`.

---

## ticket-bot

| Commande | Description | U | M | A |
|---|---|---|---|---|
| `/ticket panel` | Déployer le panneau d'ouverture | ❌ | ❌ | ✅ |
| `/ticket close` | Fermer le ticket courant | ✅* | ✅ | ✅ |
| `/ticket invite <user>` | Inviter qq dans le ticket | ✅* | ✅ | ✅ |

*✅ uniquement si le user est dans son propre ticket.

**À vérifier** :
- Un user ne peut pas fermer/inviter dans le ticket d'un autre.
- Le transcript est généré à la fermeture.
- Le salon est correctement archivé/supprimé.

---

## voice-bot

Pas de commandes slash. Bot piloté par évènements (join/leave vocal).

**À vérifier** :
- À l'entrée en vocal, une session est ouverte.
- À la sortie, une carte de session est postée avec **l'heure locale du spectateur** (fix récent).
- Aucune fuite de session après crash/redémarrage.

---

## welcome-bot

Pas de commandes slash. Bot piloté par évènements (join serveur).

**À vérifier** :
- Le message de bienvenue est posté dans le bon salon.
- Les rôles auto sont attribués.
- L'image de bienvenue est générée sans panic.

---

## Tests transverses

À exécuter une fois la matrice ci-dessus validée :

- [ ] **Permissions** : se connecter avec un compte U → vérifier qu'aucune commande ❌ n'apparaît dans l'autocomplétion.
- [ ] **Permissions** : se connecter avec un compte M → vérifier que les commandes `A` seulement (`/cleanup logs`, `/template *`, `/audit *`, etc.) sont bien masquées.
- [ ] **Audit** : chaque action mod produit une entrée dans audit-bot.
- [ ] **Logs** : aucun panic dans les logs après un cycle complet de tests.
- [ ] **Persistance** : redémarrer tous les bots → les panneaux (blackjack, roles, ticket) restent fonctionnels.
- [ ] **DB** : vérifier l'absence de lignes orphelines après tests (sessions, tickets, parties).
