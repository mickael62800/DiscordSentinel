# 🎮 Commandes Utilisateurs — Bot Sentinelle

> Commandes accessibles à tous les membres (aucune permission staff requise).
> Chaque entrée explique **ce que fait concrètement** la commande.
>
> Voir aussi : [COMMANDES_ADMIN.md](./COMMANDES_ADMIN.md) pour les commandes du staff.
> Source : `sentinel-bot/src/modules/`.

## Sommaire

| Module | Commandes |
|---|---|
| `game-bot` | `/game` |
| `community-bot` | `/parrain` |
| `progression-bot` | `/level`, `/stats` |
| `blackjack-bot` | `/blackjack` |
| `coude-bot` | 36 commandes du mini-jeu « Coup de Coude » |
| `ticket-bot` | `/ticket` |
| `confessions` | `/confess` |

---

## 🎮 game-bot — Inscription aux jeux

### `/game` — Consulter et s'inscrire aux jeux
Permet de voir la liste des jeux proposés sur le serveur et de s'y inscrire pour être
notifié / regroupé avec les autres joueurs.

| Sous-commande | Ce qu'elle fait |
|---|---|
| `list` | Affiche tous les jeux disponibles sur le serveur |
| `join` | T'inscrit à un jeu (`name` = nom du jeu) ; tu apparais comme joueur de ce jeu |
| `leave` | Te retire de la liste des joueurs d'un jeu (`name`) |

---

## 👥 community-bot — Parrainage

### `/parrain` — Parrainer un nouveau membre
Te permet de parrainer un nouveau venu sur le serveur. Le filleul doit valider le parrainage ;
une fois confirmé, le lien parrain → filleul est enregistré (et peut donner un rôle temporaire
au parrain selon la config du serveur).

---

## 📈 progression-bot — Niveaux & XP

Le serveur attribue de l'XP selon l'activité (messages, temps en vocal, ancienneté).

### `/level` — Consulter les niveaux et l'XP
Affiche la progression. Si aucun utilisateur n'est précisé, c'est la tienne qui s'affiche.

| Sous-commande | Ce qu'elle fait |
|---|---|
| `user` | Montre le niveau et l'XP d'un membre (`target`, optionnel = toi par défaut) |
| `top` | Affiche le classement du serveur par XP total (`limit` 1-25, défaut 10) |

### `/stats` — Consulter les statistiques
Montre des statistiques détaillées (activité du serveur ou d'un utilisateur précis).

---

## 🃏 blackjack-bot — Jouer au Blackjack

### `/blackjack` — Joue au Blackjack
Lance une partie de Blackjack : tu mises des coins (`mise`, minimum 10) et tu tentes
d'atteindre 21 sans dépasser pour gagner. *(Le casino multijoueur se joue surtout via le
panneau déployé par le staff, mais cette commande permet une partie solo directe.)*

---

## 🥊 coude-bot — Mini-jeu « Coup de Coude »

Mini-RPG d'économie et de combat : on gagne/perd des **coins**, on monte de niveau, on
combat les autres joueurs, on vole, on s'équipe et on grimpe au classement.

### Combat
| Commande | Ce qu'elle fait |
|---|---|
| `/coude` | Défie un autre joueur en duel ; tu mises des coins, le gagnant rafle la mise |
| `/coude-amical` | Duel d'entraînement **sans mise**, pour tester sans rien risquer |
| `/honneur` | Force au combat un joueur qui te refuse trop souvent (dette d'honneur) |
| `/vendetta` | Déclare une vendetta officielle contre un joueur pendant 7 jours |
| `/coalition` | Rejoint une coalition contre un joueur (500c, devient active à 3 membres) |

### Profil & progression
| Commande | Ce qu'elle fait |
|---|---|
| `/profil` | Affiche ton profil (niveau, stats, coins…) ou celui d'un autre joueur |
| `/resume` | Résumé des derniers mouvements de coins d'un joueur |
| `/train` | Dépense un point de statistique pour améliorer tes stats |
| `/classe` | Choisis ou change ta classe de combat (guerrier, mage, etc.) |
| `/reset-stats` | Redistribue tous tes points de stats (coûte 300 coins) |
| `/hp` | Affiche tes points de vie actuels |
| `/repos` | Récupère tous tes HP (cooldown 12h) |
| `/potion` | Utilise une potion de soin pour récupérer des HP (hors combat) |
| `/ultimate` | Affiche ou active ton pouvoir ultime (débloqué au niveau 10) |
| `/prestige` | Reset au niveau 1 contre +5% de gains permanents (niveau 25+ requis) |
| `/aide` | Donne des suggestions de jeu selon l'état actuel de ton compte |

### Économie & vol
| Commande | Ce qu'elle fait |
|---|---|
| `/voler` | Tente de pickpocket un autre joueur pour lui prendre des coins |
| `/donner` | Donne des coins ou des items à un autre joueur |
| `/assurance` | Souscris une assurance temporaire contre les pertes de combat |
| `/protection` | Abonnement anti-vol (secret, te protège des vols) |
| `/boost-voleur` | Abonnement qui augmente tes chances de réussir tes vols (secret) |
| `/cagnotte` | Affiche l'argent accumulé dans la caisse communautaire |
| `/braquage` | Tente de braquer la caisse communautaire (1×/semaine, gros risque !) |
| `/contribuer-prime` | Ajoute des coins à la prime collective d'un joueur en bonne série |
| `/prime` | Place une prime sur la tête d'un joueur (récompense pour qui le bat) |
| `/tout-ou-rien` | Mise tout ton portefeuille sur un 50/50 (1×/semaine, irréversible) |

### Social, paris & fun
| Commande | Ce qu'elle fait |
|---|---|
| `/leaderboard` | Affiche le classement Coup de Coude |
| `/pari` | Parie des coins sur l'issue du combat d'un joueur |
| `/saison` | Affiche les infos de la saison en cours |
| `/memorial` | Mémorial des « clodos » : top 10 des plus grosses pertes au tout-ou-rien |
| `/maudire` | Pose une malédiction ridicule sur un pote pendant 24h (300c) |
| `/prank` | Outils de troll communautaires |
| `/saboter` | Sabotages ciblés contre un autre joueur |
| `/no-taunts` | Active/désactive les railleries automatiques te concernant |
| `/travaux` | Effectue une tâche de prison (uniquement quand tu es en cellule) |

---

## 🎫 ticket-bot — Support

### `/ticket` — Gérer ton ticket de support
La création d'un ticket se fait généralement via le bouton du panneau déployé par le staff.

| Sous-commande | Ce qu'elle fait |
|---|---|
| `close` | Ferme le ticket du salon actuel |

---

## 🤫 confessions — Confession anonyme

### `/confess` — Poste une confession anonyme
Ouvre une fenêtre de saisie : ton message est publié anonymement dans le salon de confessions
configuré sur le serveur (ton identité n'est pas révélée aux autres membres).
