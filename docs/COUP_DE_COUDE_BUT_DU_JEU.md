# 👊 Coup de Coude — Le but du jeu

> *« C'est le jeu Discord où tu spin une roue complètement pétée chaque jour, tu te maudis entre potes, et parfois une licorne rose te rend riche. »*

---

## 🎯 En une phrase

**Coup de Coude est un jeu de combat, d'économie et de chaos entre potes**, joué entièrement via slash commands Discord. Tu te bagarres, tu voles, tu paries, tu te fais maudire, tu spin la roue, tu prestige — et tout le serveur regarde.

Ce n'est **pas** un simulateur de combat. C'est un **terrain de jeu chaotique entre amis** où tout peut virer n'importe comment.

---

## 🧭 Le boucle de gameplay

Tu démarres avec quelques **coins** (la monnaie du jeu) et un personnage de niveau 1. À partir de là :

1. **Tu joues** — combats, vols, paris, malédictions, pranks, roue du destin…
2. **Tu gagnes des coins et de l'XP** — qui te font monter de niveau.
3. **Tu débloques des paliers** — items du shop, classes, ultimates, prestige.
4. **Tu vises le top du serveur** — ou le Memorial des clodos, c'est selon.

Chaque jour, tu peux **spin la Roue du Destin** (1 fois) — la signature du jeu. 20 cases ridicules : jackpot, bisou, slip, alien, licorne. Ça tombe, tout le serveur en parle.

---

## ⚔️ Les piliers du gameplay

### 1. Le combat (`/coude`)

Tu défies un autre joueur pour une **mise** en coins. Le moteur de combat roule un d20 pour chacun, applique les classes, items spéciaux, événements de chaos, et déclare un gagnant. Le perdant cède la mise (cap sur son solde réel).

- **Combat normal** : mise réelle, conséquences réelles.
- **Combat amical** (`/coude-amical`) : sans mise, juste +20 XP au gagnant. Pour s'entraîner sans risque.
- **Surprise** : si tu as l'item, le combat se résout instantanément sans laisser le défenseur réagir.
- **Ripostes, esquives, coups traîtres, bouclier, antidote, poison, rage** : 9 attaques spéciales achetables au shop, chacune avec son effet.

Pendant un combat, **les autres joueurs peuvent parier** (`/pari`) sur le gagnant et empocher leurs gains.

### 2. L'économie

- **`/profil`** : tes coins, niveau, classe, HP, victoires/défaites.
- **`/donner`** : transférer des coins à un pote (taxe applicable).
- **`/shop`** : acheter potions, items de combat, outils de braquage, abonnements anti-vol.
- **`/cagnotte`** : la **caisse communautaire**. Tout ce qui « tombe » de l'économie (taxes, lâchetés, scams, fees) finit dedans. Redistribution hebdomadaire aux joueurs actifs.
- **`/memorial`** : le mur de la honte des joueurs ruinés au TOUT-OU-RIEN.

### 3. Le vol & la protection

- **`/voler @cible`** : tente un vol. Roll d20, dépend du niveau, de l'activité de la victime, des items « voleur » que tu possèdes.
- **`/protection`** : abonnement anti-vol (1 à 7 jours).
- **`/boost-voleur`** : abonnement pro-vol (cumulable).
- **`/braquage`** : LE gros coup. 1× par semaine. Vise la cagnotte serveur. Outils du shop = chance qui monte (cap 50 %). Réussite → jusqu'à 75 % de la caisse. Échec → **24h de prison**.

### 4. La progression

- **Niveau / XP** : montent en combattant (text/voice côté progression-bot, mais le coude a son propre système).
- **Classes** (`/classe`) : Bourrin (tank), Agile (esquive), Fourbe (vampire), Tank (statue). Chaque classe change tes stats et débloque un **ultimate** au niveau 10.
- **Paliers** (milestones) tous les 5 niveaux : +1 emplacement assurance au niveau 5, ultimate au 10, repos plus court au 15, riposte au 20, **prestige au 25**.
- **Prestige** (`/prestige`) : reset au niveau 1 mais **+5 % de gains permanents** par prestige. Cap à 5 (= 5 étoiles ⭐ + 25 % gains permanents).
- **Achievements** (`/profil`) : 30+ succès cosmétiques trackés automatiquement.

### 5. Le côté toxic-fun (interactions entre potes)

C'est ça qui différencie Coup de Coude d'un bot de combat générique.

- **`/maudire @pote`** (300c) — pose une malédiction ridicule pendant 24h (poulet, peau de banane, lenteur…)
- **`/prank @pote`** — fausse alerte braquage, faux scoop, faux DM système. Pure ambiance.
- **`/saboter @pote`** — graisser ses armes, empoisonner son wallet, lui vendre une **fausse assurance**.
- **`/vendetta @pote`** — déclare une revanche officielle. Si tu gagnes la revanche, +100 % de la mise.
- **`/contribuer-prime`** — cagnotte sur la tête d'un joueur. Quiconque le bat empoche le total + le titre **Régicide**.
- **`/honneur @lâche`** — un pote a refusé 3 fois ton défi ? Il **doit** combattre, il ne peut plus refuser.
- **`/coalition`** — 3+ joueurs se liguent contre une cible : -20 % sur ses gains pendant 48h.

### 6. La signature : la Roue du Destin (`/roue`)

1× par jour, public, animé. 20 cases :

| Type | Exemples |
|---|---|
| 💰 Loot | Jackpot 5 000c, Colis 3 items gratuits, Couronne « Roi du jour » |
| 💀 Punition | Ruine -500c, Slip (-50 % dégâts au prochain combat), Enlèvement alien (1h sans jouer) |
| 🃏 Cosmétique | Clown 24h, PQ collector inutile |
| 🦄 Ultra-rare | Licorne (+10 000c + ping @everyone), Swap de wallet |

Pourquoi c'est la signature : tout le serveur la voit, c'est racontable en une phrase (« le jeu où tu spin une roue débile chaque jour »), ça crée un **rituel quotidien**.

### 7. Les moments « OH SHIT » (chaos)

- **Événements chaos Mythiques** (très rares) : Licorne rose, Étoile filante, Bombe nucléaire, Aliens qui abductent les deux joueurs… Chacun a un effet absurde et unique. Ping serveur quand ça tombe.
- **Daily chaos** (configurable) : chance qu'un combat normal se transforme en happy hour, bloodbath, hyperinflation…
- **Saisons** (`/saison`) : tous les 90 jours, un thème change l'équilibre (Saison du Vol, du Tank, du Chaos…).

---

## 🎭 Les filets de sécurité

Le jeu peut être brutal. Plusieurs garde-fous évitent les spirales de la mort :

| Filet | Effet |
|---|---|
| **Bouclier malchance** | Première défaite du jour ×0.5 sur la perte + win streak préservée |
| **Filet de sécurité** | Sous 50 coins → 3 jours où les pertes sont divisées par 2 et les gains de paris ×1.5 |
| **Cowardice relief** | Refuser un combat n'incrémente pas la lâcheté si la cible est < 20 % HP |
| **Travaux en prison** | Pendant les 24h post-braquage raté, gain de coins via tâches (au lieu d'être muet) |
| **Tournoi mensuel** | Live samedi par mois, bracket élim — moment communautaire |

---

## 🏆 Les objectifs long-terme

| Objectif | Comment l'atteindre |
|---|---|
| **Top wallet du serveur** | Combats + vols + paris + braquages réussis. Affiché dans `/leaderboard`. |
| **Prestige V** | Niveau 25 → prestige × 5. Étoiles permanentes. |
| **Tous les achievements** | 30+ succès trackés (`/profil`). |
| **Roi du Tournoi** | Gagner le tournoi mensuel live. |
| **Régicide** | Briser la série de 5 victoires d'un joueur sous bounty. |
| **Memorial des clodos** | Perdre tout son wallet au TOUT-OU-RIEN — leaderboard de la honte. |

---

## 🎨 Le ton

**Le jeu assume le ridicule.** Les textes sont volontairement débiles, les commentaires de combat absurdes (« Bob trébuche sur une écharde émotionnelle »), les events font des références mèmes. C'est un **Kaamelott dopé aux RedBull** — pas un simulateur sérieux.

Si un joueur raconte le jeu à un nouveau, il doit pouvoir lâcher en une phrase :

> *« C'est le jeu Discord où tu spin une roue complètement pétée chaque jour, tu te maudis entre potes, et parfois une licorne rose te rend riche. »*

---

## 📚 Pour aller plus loin

- **Toutes les commandes** : voir `COUP_DE_COUDE_COMMANDES.md`
- **Guide express** : voir `COUP_DE_COUDE_POUR_LES_NULS.md`
- **Roadmap & features** : voir `amélioration/COUPE_AMELIORATIONS.md`
- **Architecture technique** : voir `amélioration/COUDE_ARCHITECTURE_AUDIT.md`

---

*Tagline officielle : « Coup de Coude · Le chaos gagne toujours. »*
