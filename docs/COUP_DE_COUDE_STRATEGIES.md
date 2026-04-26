# 🎯 Coup de Coude — Stratégies & conseils

> Le jeu est ridicule, le RNG aussi. Mais il y a des choix qui paient et d'autres qui ruinent. Voici ce que les joueurs malins font (et ce qu'ils évitent).

---

## 🌱 Phase 1 — Tu débutes (niveau 1 à 5)

### Les 4 réflexes à avoir dès le départ

1. **Choisis ta classe le jour 1** (`/classe`) — le 1ᵉʳ choix est gratuit. Sans classe, tu joues sans bonus passif.
2. **Lis `/aide`** — la commande te dit ce que tu peux faire selon ton état.
3. **Mise petit au début** — 50-100c max les premiers combats. Si tu perds 500c quand tu en as 800, tu es coincé.
4. **Garde toujours `/repos` dispo** — ne lance pas un combat avec 30 % HP « pour voir ». Tu vas perdre et ne plus pouvoir rejouer pendant 12h.

### Les pièges du débutant

- ❌ **Tout miser au premier combat** : tu ne connais pas encore ton matchmaking handicap, les attaques spéciales, les events chaos. Tu perds, tu décourages.
- ❌ **Ignorer le shop** : un seul `bouclier` (250c) peut sauver ton wallet en défense. C'est le meilleur ratio coût/utilité au début.
- ❌ **Refuser tous les combats** : la lâcheté s'accumule (`cowardice_count`). À 5+, tu apparais dans le top des lâches sur le `/leaderboard`. Préfère perdre un petit combat que d'accumuler 5 refus.
- ❌ **Spammer `/voler`** : cooldown 30 min, et si tu rates tu paies une pénalité. 1-2 vols ciblés > 10 vols random.

### Quoi acheter en priorité au shop

| Item | Prix | Pourquoi |
|---|---|---|
| 🛡️ Bouclier | 250c | Défense, +DEF temporaire — survit aux gros attaquants |
| 🧪 Potion de soin | 80c | +30 HP, à toujours avoir 2-3 en stock |
| 💥 Explosion | 200c | En défense, annule le combat (les 2 perdent 50% mise — souvent mieux que prendre 100%) |
| 😡 Rage | 100c | +ATK temporaire, pour tes propres attaques |

> 💡 **Astuce** : achète **avant** d'avoir besoin. Un combat se résout en quelques secondes — pas le temps de courir au shop.

---

## ⚔️ Stratégies de combat

### Choisir ta cible

- **Underdog hunting** : défier un joueur **3+ niveaux en dessous** déclenche le **handicap matchmaking** : -X % ATK pour toi. Si tu gagnes quand même, **mise doublée + XP ×2** (Giant Killer).
- **Equal match** : le mieux pour de l'XP régulier, peu de risque.
- **Stealing big** : viser un joueur riche, mais qui a sûrement des items défensifs. Lance un `/voler` plutôt qu'un `/coude` si c'est juste pour ses coins.
- **Pas de duel sans enjeu** : le bot bloque les défis contre un joueur à 0 coin.

### Les attaques spéciales (qui font mal)

| Item | Effet | Quand l'utiliser |
|---|---|---|
| **Surprise** (300c) | Auto-win si la cible n'a pas d'**Explosion** | Sur cible sans inventaire visible |
| **Coup traître** (350c) | -DEF cible massivement | Contre un Tank ou cible HP plein |
| **Double coup** (250c) | 2 attaques au lieu d'1 | Vs cible HP basse, finir le combat |
| **Mindgame** (150c) | Brouille les rolls cible | Vs joueur avec gros ATK |
| **Poison** (300c) | Dégâts par tour | Combats longs, vs Tank |

### Les défenses qui sauvent

| Item | Effet | Quand cliquer le bouton « Objet » |
|---|---|---|
| **Bouclier** (250c) | +DEF gros bonus | Cible avec gros ATK |
| **Antidote** (150c) | Annule poison entrant | Si tu vois Poison dans le défi |
| **Rage** (100c) | +ATK quand tu défends | Si tu veux retourner le combat |
| **Explosion** (200c) | Annule le combat, -50% mise pour les 2 | Si tu sais que tu vas perdre 100% |

> 💡 **Règle d'or** : **toujours avoir au moins 1 Explosion en stock**. C'est ton « stop-loss » universel.

---

## 🥷 Stratégies de vol

### Les 3 vrais critères

1. **L'AFK** : une victime AFK ne peut pas se défendre dans les 60s → vol auto-réussi avec malus.
2. **Le solde de la cible** : tu voles 15-25 % de son wallet. Plus elle est riche, mieux c'est.
3. **Tes outils boost** : `/boost-voleur` cumulatif, +5 à +25 au roll. **Invisible** pour la victime.

### Le combo gagnant

```
1. Achète "crochet" et "drone_espion" (boost-voleur)
2. Active une protection "chien_garde" sur toi (anti-vol retour)
3. Vise un joueur riche, idéalement actif vers 14h ou 22h (peu de monde)
4. /voler → le bot fait son job
```

### Pièges à éviter

- ❌ **Voler quelqu'un avec < 50c** : le bot bloque (« même les voleurs ont des principes »).
- ❌ **Voler en boucle la même cible** : son streak de victime augmente, elle reçoit des compensations.
- ❌ **Oublier ta propre protection** : un voleur volé est un meme.

---

## 💰 Stratégies économiques

### La hiérarchie des sources de revenus

| Source | Risque | Gain attendu | Fréquence |
|---|---|---|---|
| `/coude` (combat équilibré) | Moyen | + ou - mise | Illimité |
| `/voler` ciblé | Bas-Moyen | 15-25% du wallet cible | 1× / 30 min |
| `/pari` sur combats des autres | Bas | +50-100% si tu gagnes | Illimité |
| `/braquage` (cagnotte serveur) | Très haut | 30-75% de la cagnotte | 1× / semaine |
| `/tout-ou-rien` | Maximal | ×2 ou -80 % | 1× / semaine |
| Roue du destin | Aléatoire | -500 à +10 000c | 1× / jour |

### Le pari intelligent

- `/pari @combattant mise` quand quelqu'un d'autre se bat. Tu n'as **rien à perdre côté HP**.
- **Vise les outsiders** : un underdog sous handicap qui **gagne** te donne un meilleur multiplier.
- **Pas tout sur un combat** : éclate sur 3-4 combats simultanés.

### Le braquage — le big play

- **Attends que la cagnotte soit grosse** (`/cagnotte` avant de tenter). Vise > 5 000c minimum.
- **Achète 4-6 outils** avant de tenter (cap 50 % chance). Sans outils = 5 % chance, suicide.
- **Préviens personne** : annoncer tes plans, c'est inviter quelqu'un à te griller.
- **Si tu rates** : 24h de prison, mais **`/travaux`** te rapporte ~500c sur la durée. Ne désespère pas.

### Le tout-ou-rien — le suicide volontaire

- ⚠️ **Ne joue jamais si tu es ton joueur principal**. Tu es là pour 80 % de ton wallet en l'air.
- Joue uniquement quand tu as un **gros wallet hérité** que tu peux te permettre de perdre, ou pour tenter un comeback désespéré.
- Le **Memorial** (`/memorial`) garde la trace des plus grosses ruines. Honte ou gloire selon le ton.

---

## 🛡️ Tirer parti des filets de sécurité

Le jeu a 3 garde-fous que **peu de joueurs exploitent** :

### 1. Le bouclier malchance du jour

- Première défaite quotidienne = perte ×0.5, **win streak préservée**.
- **Stratégie** : si tu sens que tu vas perdre un combat important, lance d'abord un petit combat « test » que tu vas perdre — il consomme le bouclier sur peu de coins. Le combat important sera ensuite à perte normale.
- ❌ **À l'inverse** : si tu as une grosse mise et 0 défaite aujourd'hui, ne perds pas exprès — le shield est précieux pour le moment où ça compte.

### 2. Le filet de sécurité

- Activé automatiquement sous 50 coins. Pendant **3 jours** : pertes ÷2, paris gagnants ×1.5.
- **Stratégie** : si tu es proche de la ruine, **provoque la chute** (ex. `/tout-ou-rien` ou un gros combat perdu) pour passer sous 50c. Tu déclenches le filet et tu peux remonter avec moitié moins de risque.
- ⚠️ Ce trick est **borderline** — utilisable mais éthiquement discutable entre potes.

### 3. Cowardice relief (HP bas)

- Refuser un combat quand tu es < 20 % HP **n'incrémente pas la lâcheté**.
- **Stratégie** : si quelqu'un te défie et que tu es low HP, refuse sans honte. Repose-toi puis riposte avec `/coude` quand tu es plein.

---

## 🎓 Stratégies par classe

### 🥊 Bourrin — Tank offensif

- **Bonus** : gros ATK, gros HP.
- **Faiblesse** : rolls instables (du tout au tout).
- **Stratégie** : `/coude` direct, `/braquage` (besoin de gros HP), `/maudire` les Agiles qui esquivent.
- **Ultimate (niveau 10)** : **Échange de carcasses** — swap ton HP avec l'adversaire avant le combat. Pose un combat avec 5 HP → l'adversaire hérite de tes 5 HP, toi des siens.

### 🏃 Agile — Esquive et finition

- **Bonus** : esquive partielle, rolls plus fiables.
- **Faiblesse** : DEF moyenne, vulnérable aux Coups traîtres.
- **Stratégie** : `/voler` (bonus naturel), combats à mise moyenne pour empiler les wins, vise les Bourrins (qui rate leurs rolls).
- **Ultimate (niveau 10)** : **Pile ou face** — combat instantanément résolu sur un 50/50 pur. Buff secret +5 % si tu as l'ult.

### 🗡️ Fourbe — Vampire & vol

- **Bonus** : vampirise les dégâts subis, gros bonus `/voler`.
- **Faiblesse** : HP de base bas.
- **Stratégie** : économie pure — **vol intensif**, sabotage (`/saboter empoisonner`), vendetta sur quiconque te touche.
- **Ultimate (niveau 10)** : **Le Fuyard** — vole la mise AVANT le combat et te casses. Cooldown 2 semaines (vs 1 pour les autres). Abusé.

### 🛡️ Tank — Mur infranchissable

- **Bonus** : énorme DEF, gros HP.
- **Faiblesse** : ATK molle, combats longs.
- **Stratégie** : combats d'usure, défense items (Bouclier, Antidote), sois la cible — les attaquants se cassent les dents.
- **Ultimate (niveau 10)** : **Statue** — ne fait aucun dégât, mais n'en prend pas non plus. Gagne par forfait au bout de 10 rounds. Le troll ultime.

---

## 🎭 Stratégies social-toxic (entre potes)

### Le combo malédiction + sabotage

```
1. /maudire @cible chicken (300c) — son pseudo devient @Cible le Poulet 24h
2. /saboter @cible empoisonner (400c) — 10% de ses 3 prochains gains pour toi
3. Tu attends, il joue, tu encaisses
```

### La vendetta calibrée

- `/vendetta @cible` quand tu **sais** que tu peux le battre (level proche, classes favorables, items prêts).
- Si tu gagnes la revanche → **+100 % gain** — gros score.
- Si tu reperds → ton nom devient `@toi le Bourreau de @cible` 7 jours. Ça pique.

### La coalition

- 3+ joueurs se liguent (`/coalition @cible`, 500c chacun) → cible perd 20 % gains pendant 48h.
- **Cible peut casser** la coalition en battant **un seul** des conspirateurs en `/coude` direct.
- **Stratégie** : monte une coalition sur le joueur dominant du serveur. Soit il bat l'un de vous (et la coalition tombe), soit il subit 48h de pénalité.

### La prime collective

- Quiconque gagne 5 combats d'affilée → bounty s'ouvre automatiquement (1 000c initial).
- `/contribuer-prime @cible 500` pour gonfler le pot.
- Quiconque bat la cible **empoche tout + le titre Régicide**.
- **Stratégie** : si un joueur est sur une streak insolente, mets une prime — ça motive le serveur à le punir.

---

## 🎰 La Roue du Destin — comment l'optimiser

`/roue` 1 fois par jour. Pas de skill, mais quelques choses à savoir :

- **Spin tous les jours** — c'est gratuit. La case 13 (Blanche) peut tomber, mais les ultra-rares (Licorne, Roi du monde, Couronne) en valent la peine.
- **Profite des cases buff** : si tu tombes sur **Ceinture noire** (+50% dégâts) ou **Sieste** (+20% regen HP), planifie un gros combat dans la fenêtre.
- **Évite les combats si tu tombes sur Slip** (-50% dégâts) ou **PQ** (rien). Joue passif.
- **Si Mue** (changement de classe 24h) : teste la nouvelle classe, tu apprends potentiellement quelque chose pour ton choix permanent.

---

## 📈 Stratégies long-terme

### Niveau 1 → 10 — Construction

- Distribue tes points sur **DEF d'abord** (survie). Puis ATK.
- Achète l'inventaire de base (Bouclier, Potions, Explosion, Antidote).
- Choix de classe stable.
- Vise la **première streak de 5** pour déclencher la première bounty sur ta tête (le serveur viendra te chercher = activité garantie).

### Niveau 10 → 25 — Spécialisation

- Débloque ton **ultimate** (`/ultimate`).
- Affine ton inventaire (items niveau, premium au shop).
- Lance ta **première vendetta** sur un rival que tu n'as jamais battu.
- Fais ton **premier braquage** (besoin de gros HP).

### Niveau 25+ — Prestige

- `/prestige` reset au niveau 1, mais **+5 % gains permanents**.
- Cap : 5 prestiges = +25 % gains permanents + 5 ⭐.
- **Stratégie** : prestige le plus tôt possible (niveau 25). Le bonus de gain est permanent et compound.

---

## 🏆 Optimiser pour les achievements

30+ succès trackés (`/profil`). Quelques-uns valent le détour pour la prestige sociale :

| Achievement | Comment l'obtenir |
|---|---|
| Survivant | Gagner un combat sous 5 % HP |
| Giant Killer | Battre un adversaire 5 niveaux au-dessus (×3) |
| Serial Voleur | 10 vols réussis |
| Roi du Chaos | Déclencher 20 events chaos différents |
| Millionnaire | 100 000 coins |
| Patrouilleur | Bloquer 10 vols avec items actifs |

---

## ⚠️ Les pièges vicieux

### 1. La fausse assurance

`/assurance` a ~5 % de chance d'être un **scam** (cosmétique). Tu paies, l'assurance ne s'active pas. Si tu vois le message « ton contrat était un scam » au moment d'une perte, c'est ça.

### 2. Le sabotage « fausse assurance » (`/saboter fausse_assurance`)

Quelqu'un peut te vendre une fausse assurance pour **500c** (côté lui). Si tu perds avec, l'assurance est annulée + 200c partent vers le saboteur. **Vérifie toujours qui tu côtoies**.

### 3. La banane

La malédiction `banana` te fait **rater 30 % de tes d20**. Si tu enchaînes des combats et que tu rolls 1, 2, 1 → un pote t'a maudit. `/profil` t'indique tes malédictions actives.

### 4. La prison post-braquage raté

24h pendant lesquelles tu **ne peux que `/travaux`**. Pas de `/coude`, `/voler`, etc. Si tu es sur une bonne streak, **ne tente pas le braquage** au mauvais moment.

### 5. Le tout-ou-rien — l'unique vraie ruine

80 % de ton wallet en un clic. **Lis bien le message** avant de cliquer le bouton de confirmation. C'est irréversible et tu finis au Memorial.

---

## 🥷 Tactiques ninja

### 1. Le drainage du wallet rival

```
1. /pari sur ses adversaires
2. /voler quand il est occupé en combat
3. /saboter empoisonner pour récupérer 10% de ses gains
4. /vendetta quand il est faible
```

### 2. Le farming économique

```
1. /repos full HP
2. /coude petite mise sur des cibles équilibrées
3. /pari sur d'autres combats simultanément
4. Spin /roue
5. Refais le cycle
```

### 3. L'investissement long-terme

```
1. Stack des items shop quand tu as des coins
2. Achète des protections longues (5-7 jours) pour bloquer les vols
3. Économise pour le tout-ou-rien (mais joue-le que tu as un gros surplus)
4. Vise la prestige V (5 ⭐)
```

---

## 💎 Les golden rules

1. **Ne joue jamais ton wallet entier** sauf si tu acceptes de perdre.
2. **Toujours 1 Explosion en stock** — assurance universelle.
3. **`/repos` avant tout combat important**.
4. **Lis `/aide` quand tu hésites** — les tips sont contextuels.
5. **`/profil` régulièrement** — checke tes malédictions, sabotages, items actifs.
6. **Le shop est ton ami** — ne joue pas sans inventaire.
7. **Pas de duel sans enjeu** — un joueur à 0 coin ne peut pas perdre, donc ne te rapporte rien.
8. **La cagnotte gonfle** — surveille `/cagnotte`, c'est ton braquage potentiel.
9. **Les pranks et malédictions ne tuent pas** — accepte d'être trolled, rendis la pareille.
10. **Le chaos gagne toujours**. Mythic events, sabotage, swap, alien… tu ne contrôles pas tout. **Adapte-toi**.

---

*« Coup de Coude · Le chaos gagne toujours. »*

*Pour les commandes : voir `COUP_DE_COUDE_COMMANDES.md` · Pour la philosophie : voir `COUP_DE_COUDE_BUT_DU_JEU.md`.*
