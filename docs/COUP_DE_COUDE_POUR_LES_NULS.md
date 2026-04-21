# Coup de Coude — guide express

Bienvenue ! Le principe : tu te bagarres contre d'autres joueurs, tu voles des coins, tu montes de niveau, tu achetes des items. Tout se joue en slash commands Discord.

## Les 5 commandes à connaître

- `/coude @user mise` — défie quelqu'un. Le gagnant empoche la mise du perdant. Mise à 0 autorisée (juste pour le fun).
- `/voler @user` — pique des coins à quelqu'un d'AFK ou actif. Si tu rates tu paies une pénalité. Cooldown entre les vols.
- `/profil` — ton fiche : HP, niveau, classe, coins, stats.
- `/shop` — achète potions (soin), bouclier (défense), explosion (annuler un combat), surprise (auto-win) et plein d'autres. Défausse-toi quand on t'attaque en choisissant un item.
- `/repos` — récupère tes HP d'un coup (cooldown 12h).

## Astuces de survie

1. **Reste au-dessus de 40% HP** sinon tu peux plus combattre (et on peut plus te défier). `/repos` ou potions pour remonter.
2. **En défense**, le bot te propose d'utiliser un item. Si tu as une **Explosion**, tu peux annuler le combat (les deux perdent 50% de la mise, souvent mieux que prendre une défaite).
3. **Les vols** : plus ta victime a de coins, plus tu gagnes. Mais elle peut activer une **Protection** (shop) pour bloquer.
4. **Les classes** (`/classe`) changent tes stats : Guerrier = tank, Fourbe = vampirise les dégâts, Voleur = bonus sur `/voler`, etc.
5. **Attention aux streaks** : 3 défaites ou 3 vols subis d'affilée = le bot te rebaptise avec un emoji humiliant. `/no-taunts on` si tu veux échapper aux pseudos modifiés.

## La caisse commune (cagnotte)

Chaque fois que quelqu'un fuit un combat (lâcheté), paie une taxe de don ou explosion, les coins tombent dans une **caisse communautaire**. Elle grossit au fil des jours.

- **Tournoi hebdo** : chaque dimanche 23h UTC, le joueur avec le plus de gains nets sur la semaine empoche un pourcentage de la caisse (`/coude` → page tournoi dans la web UI).
- **Braquage** (`/braquage`) : toi-même tu peux tenter de voler la caisse ! Une fois par semaine max. Plus tu as d'outils du shop (masque, pied-de-biche, crochet, explosif, drone, équipe de pros…) plus ta chance monte, cap à 50%. Si tu réussis → tu empoches jusqu'à 75% de la caisse. Si tu rates → **24h de prison** + tu perds une partie de tes outils (aucune action pendant ce temps).

Le braquage est le gros coup : risqué mais jackpot. Assure tes arrières avec quelques potions avant de tenter.

Amuse-toi bien. Si le bot dit "Echec de l'interaction", re-clique — c'est juste Discord qui boude.
