# 🥊 Coup de Coude — Liste des commandes

> Toutes les commandes slash du mini-jeu « Coup de Coude » (`coude-bot`), regroupées par thème.
> Descriptions reprises du code (`sentinel-bot/src/modules/coude/commands/`).
>
> Pour comprendre les mécaniques, voir [COUP_DE_COUDE_JEU.md](./COUP_DE_COUDE_JEU.md).
>
> ℹ️ Le jeu a été **simplifié** : le méta-jeu lourd (braquage/prison, assurances,
> vendetta/coalition, prestige/ultimate, paris, saisons, sabotages…) a été retiré
> pour rester **fun & simple**. 20 commandes au total.

## Combat
| Commande | Ce qu'elle fait |
|---|---|
| `/coude` | Défie un autre joueur en duel ; tu mises des coins, le gagnant rafle la mise |
| `/coude-amical` | Duel d'entraînement **sans mise**, pour tester sans rien risquer |

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
| `/aide` | Suggestions de jeu selon l'état actuel de ton compte |

## Économie
| Commande | Ce qu'elle fait |
|---|---|
| `/voler` | Tente de pickpocket un autre joueur pour lui prendre des coins |
| `/donner` | Donne des coins ou des items à un autre joueur |
| `/tout-ou-rien` | Mise tout ton portefeuille sur un 50/50 (1×/semaine, irréversible) |

## Boutique
| Commande | Ce qu'elle fait |
|---|---|
| `/shop` | Boutique Coup de Coude — items d'attaque et de défense |

## Social & fun
| Commande | Ce qu'elle fait |
|---|---|
| `/leaderboard` | Affiche le classement Coup de Coude (avec bouton « Mettre à jour ») |
| `/memorial` | Mémorial des « clodos » : top 10 des plus grosses pertes au tout-ou-rien |
| `/prank` | Outils de troll communautaires (pour embêter tes potes pour rigoler) |
| `/no-taunts` | Active/désactive les railleries automatiques te concernant |

## Administration
| Commande | Ce qu'elle fait | Permission |
|---|---|---|
| `/taunts-channel` | Configure le salon des railleries automatiques | Admin |
