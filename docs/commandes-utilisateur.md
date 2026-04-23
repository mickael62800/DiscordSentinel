# Commandes Discord disponibles aux membres

Ce guide liste les slash commands que tout membre peut utiliser sur le serveur. Les commandes de jeu (`/blackjack`, `/game`, `/coude`, `/pari`, `/braquage`, `/shop`, `/profil`, `/cagnotte`, etc.) ne sont pas documentees ici.

---

## Niveaux et XP

### `/level`

Consulter les niveaux et l'XP.

- `/level user [target]` — affiche le niveau d'un utilisateur (par defaut : vous-meme).
  - `target` (utilisateur, optionnel)
- `/level top [limit]` — classement global par XP total.
  - `limit` (entier 1-25, defaut 10)
- `/level top_text [limit]` — classement XP texte.
  - `limit` (entier 1-25, defaut 10)
- `/level top_voice [limit]` — classement XP vocal.
  - `limit` (entier 1-25, defaut 10)

### `/stats`

Consulter les statistiques d'activite.

- `/stats user [target]` — stats d'un utilisateur (messages envoyes, temps en vocal, etc.).
  - `target` (utilisateur, optionnel)
- `/stats server` — stats globales du serveur.
- `/stats top [limit]` — classement des membres les plus actifs.
  - `limit` (entier 1-25, defaut 10)

---

## Tickets de support

### `/ticket`

- `/ticket close` — ferme le ticket ouvert dans le salon actuel (utilisable uniquement dans un salon de ticket).

> La creation d'un ticket se fait via le bouton du panneau deploye par le staff dans un salon dedie — pas de slash command a utiliser.

---

## Parrainage

### `/parrain <membre>`

Parrainer un nouveau membre du serveur. Le filleul doit confirmer via un bouton avant que le parrainage soit enregistre.

- `membre` (utilisateur, requis)
- Cooldown : 30 secondes entre chaque utilisation.

---

## Moderation cote membre

### `/appeal`

Contester une sanction recue. Un ticket est cree automatiquement pour echanger avec le staff.

Aucun argument.

---

## Salons vocaux temporaires

Les salons vocaux temporaires se gerent via les **boutons** du panneau integre au chat du salon (pas de slash command). Fonctions disponibles :

- Renommer le salon
- Verrouiller / deverrouiller
- Definir la limite d'utilisateurs
- Masquer le salon
- Changer le statut
- Transferer la propriete
- Gerer la whitelist / bans

Il suffit de rejoindre le salon de creation (generateur) pour obtenir son propre salon temporaire et acceder au panneau.

---

## Panneaux de roles

Les roles optionnels (notifications, centres d'interet, etc.) s'attribuent via les **panneaux de roles** deployes par les admins. Clique simplement sur le bouton ou selectionne ton role dans le menu deroulant — aucune commande a taper.
