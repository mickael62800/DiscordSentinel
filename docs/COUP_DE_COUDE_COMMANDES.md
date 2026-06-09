# 🥊 Coup de Coude — Liste des commandes

> Toutes les commandes slash du mini-jeu « Coup de Coude » (`coude-bot`), regroupées par thème.
> Descriptions reprises du code (`sentinel-bot/src/modules/coude/commands/`).
>
> Pour comprendre les mécaniques, voir [COUP_DE_COUDE_JEU.md](./COUP_DE_COUDE_JEU.md).

## Combat
| Commande | Ce qu'elle fait |
|---|---|
| `/coude` | Défie un autre joueur en duel ; tu mises des coins, le gagnant rafle la mise |
| `/coude-amical` | Duel d'entraînement **sans mise**, pour tester sans rien risquer |
| `/honneur` | Force au combat un joueur qui te refuse trop souvent (dette d'honneur) |
| `/vendetta` | Déclare une vendetta officielle contre un joueur pendant 7 jours |
| `/coalition` | Rejoint une coalition contre un joueur (500c, devient active à 3 membres) |

## Profil & progression
| Commande | Ce qu'elle fait |
|---|---|
| `/profil` | Affiche ton profil (niveau, stats, coins…) ou celui d'un autre joueur |
| `/resume` | Résumé des derniers mouvements de coins d'un joueur |
| `/train` | Dépense un point de statistique pour améliorer ton ATK ou ta DEF |
| `/classe` | Choisis ou change ta classe de combat (bourrin, agile, fourbe, tank) |
| `/reset-stats` | Redistribue tous tes points de stats (coûte 300 coins) |
| `/hp` | Affiche tes points de vie actuels |
| `/repos` | Récupère tous tes PV (cooldown 12h) |
| `/potion` | Utilise une potion de soin pour récupérer des PV (hors combat) |
| `/ultimate` | Affiche ou active ton pouvoir ultime (débloqué au niveau 10) |
| `/prestige` | Reset au niveau 1 contre +5 % de gains permanents (niveau 25+ requis) |
| `/aide` | Suggestions de jeu selon l'état actuel de ton compte |

## Économie & vol
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
| `/travaux` | Effectue une tâche de prison (uniquement quand tu es en cellule) |

## Boutique
| Commande | Ce qu'elle fait |
|---|---|
| `/shop` | Boutique Coup de Coude — items d'attaque, défense ou braquage |

## Social, paris & fun
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

## Administration
| Commande | Ce qu'elle fait | Permission |
|---|---|---|
| `/taunts-channel` | Configure le salon des railleries automatiques | Admin |
