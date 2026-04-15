# ⚔️ Coup de Coude — Guide du joueur

Bienvenue dans **Coup de Coude**, le mini-jeu de bagarre chaotique du serveur.
Tu défies d'autres joueurs en 1v1 pour miser des coins, grimper en niveau,
équiper des objets, parier sur les combats des autres et voler ceux qui
baissent la garde.

> 🎯 **Tout le jeu se joue via des commandes slash** (`/commande`) dans les
> salons dédiés. Aucun menu, aucun client à installer.

---

## 🚀 Démarrage rapide (5 minutes)

1. **`/profil`** — crée ton personnage (niveau 1, Bourrin par défaut)
2. **`/classe`** — choisis ta vraie classe (gratuit la première fois)
3. **`/shop`** — regarde les objets disponibles
4. **`/coude @joueur 50`** — défie quelqu'un pour 50 coins
5. **`/pari combat:<id> sur:@joueur amount:30`** — parie sur un combat en cours

---

## 💰 L'économie : les coins

Tu commences avec un solde de coins géré dans le **wallet partagé** de
Sentinel (le même que pour le Blackjack). Tu gagnes des coins en :

- Gagnant des combats (**70 % à 100 % de la mise adverse**, selon la marge de
  victoire — voir section Combat)
- Pariant correctement sur le combat d'un autre
- Recevant des dons d'un pote via `/donner`
- Votre job quotidien Discord (si activé côté serveur)

Tu en perds en :

- Perdant un combat (**60 % à 100 % de ta mise**)
- Pariant sur le perdant
- Achetant des objets au shop
- Payant une assurance
- Te faisant voler via `/voler`
- Laissant expirer un défi en attente (pénalité **20 % de la mise, min. 1 coin**)

> ⚠️ **Règle anti-exploit** : si tu gagnes contre quelqu'un qui n'a que 30
> coins alors que la mise était 100, tu ne récupères que **30 coins** (les
> coins ne se créent pas à partir de rien). Le bot t'affiche un avertissement
> au moment du défi si l'adversaire est plus pauvre que la mise.

---

## 🎭 Les classes (4 archétypes)

À ton premier `/classe`, tu choisis gratuitement. Pour changer ensuite :
**500 coins + cooldown de 7 jours**.

| Classe | Emoji | ATK base | DEF base | Croissance / niveau | Passif |
|---|---|---|---|---|---|
| **Bourrin** | 💪 | 25 | 8 | +4 ATK / +1 DEF | **Berserker** : +25 % ATK quand HP ≤ 30 % |
| **Agile** | 🏃 | 12 | 18 | +2 ATK / +3 DEF | **Esquive** : 15 % de chance d'esquiver un round |
| **Fourbe** | 🗡️ | 18 | 14 | +3 ATK / +2 DEF | **Vampirisme** : soigne 10 % des dégâts infligés + vol 20 % de la mise en bonus |
| **Tank** | 🛡️ | 8 | 25 | +1 ATK / +4 DEF | **Blindage** : -5 dégâts reçus par round (min 1) |

> ⚙️ **Exception Tank vs Tank** : les deux blindages s'annulent pour éviter
> les combats interminables en mode "1 dégât par round".

**Quel style pour quel profil ?**
- **Bourrin** = high-risk, high-reward, très fort quand le combat s'éternise
- **Agile** = jeu défensif, excelle contre les Bourrins
- **Fourbe** = maximisation des gains, faible survie
- **Tank** = durée de vie énorme, peu de dégâts infligés, idéal vs Bourrin

---

## 📈 Progression : XP, niveaux, stats

### Niveaux (1 → 25)

L'XP cumulée pour atteindre le niveau `N` = `50 × N² + 50 × N`

| Niveau | XP total | Titre |
|---|---|---|
| 1 | 100 | Débutant |
| 5 | 1 500 | Bagarreur |
| 10 | 5 500 | Guerrier |
| 15 | 12 000 | Vétéran |
| 20 | 21 000 | Champion |
| 25 | 32 500 | Inarrêtable (MAX) |

**Tu gagnes de l'XP en :**
- Gagnant un combat : **+15 XP**
- Perdant un combat : **+5 XP** (oui, même les défaites)
- **Giant Killer** (underdog qui bat un adversaire 3+ niveaux au-dessus) : **+30 XP** (×2)
- Volant réussi : **+5 XP** (`/voler`)
- Bloquant un vol : **+3 XP**

### Points de stats (1 par niveau)

À chaque level-up tu gagnes **1 point** à dépenser via **`/train`** :

- **`/train stat:attaque`** → +1 ATK effective
- **`/train stat:defense`** → +1 DEF effective (et donc +2 HP max)

**Reset complet** via **`/reset-stats`** (coûte **300 coins**, re-crédite tous
tes points d'un coup — utile si tu veux changer d'optimisation).

### Formule de stats effectives

```
ATK effective = base_atk_classe + (niveau - 1) × atk_growth + points ATK
DEF effective = base_def_classe + (niveau - 1) × def_growth + points DEF
HP max        = 100 + DEF effective × 2
```

**Exemple** : un Tank niveau 10 sans points manuels a
- ATK = 8 + 9 × 1 = **17**
- DEF = 25 + 9 × 4 = **61**
- HP max = 100 + 61 × 2 = **222**

---

## ❤️ Les HP (Points de Vie)

Tu as une barre de HP qui encaisse les coups des combats. Contrairement aux
coins, **tes HP ne se remettent pas à zéro entre deux combats**.

### Voir tes HP

**`/hp`** — affiche ta barre de vie actuelle + HP max + %.

### Récupération passive (automatique, gratuite)

Tes HP se régénèrent automatiquement par paliers, toutes les ~5 minutes :

| % HP actuels | Taux de régen |
|---|---|
| 0 – 25 % | **100 HP/h** |
| 25 – 50 % | **50 HP/h** |
| 50 – 75 % | **30 HP/h** |
| 75 – 100 % | **10 HP/h** |

> Plus tu es bas en PV, plus tu régénères vite. **Skip pendant un combat en
> cours** : tes HP ne bougent pas tant que tu as un défi en `pending` /
> `betting` / `resolving` pour éviter les conflits.

### Récupération active

- **`/repos`** — restauration complète instantanée, **cooldown 12 h**
- **`/potion type:potion_soin`** — +30 HP, consomme une potion d'inventaire
- **`/potion type:potion_majeure`** — +80 HP, consomme une potion majeure

---

## ⚔️ Lancer un combat — `/coude`

### Syntaxe complète

```
/coude cible:@user mise:100 special:<optional_item>
```

- **`cible`** (obligatoire) : le joueur à défier
- **`mise`** (obligatoire) : entre `min_bet` et `max_bet` (configuré par
  serveur, typiquement **10 → 500 coins**)
- **`special`** (optionnel) : un objet offensif consommé au lancement
  (rage, surprise, mindgame, double_coup, poison, coup_traitre)

### Le flow d'un combat normal

1. **Tu lances `/coude @Bob 100`**
2. Le bot te montre un **pré-confirmation** avec :
   - Tes PV actuels (et une alerte si tu es bas en vie)
   - Une alerte si Bob n'a pas les 100 coins (tu ne gagneras que ce qu'il a)
   - Un avertissement de handicap si l'écart de niveau est ≥ 3
3. Tu cliques **Confirmer** → le combat est créé, message public posté dans
   le salon combats
4. **Bob reçoit une notification** dans le salon notifications (pingé)
5. Bob a **24 h** pour cliquer **Accepter** (ou **Objet** pour choisir une
   défense) ou **Refuser**
6. S'il accepte → **phase de paris** ouverte pendant **5 minutes** (les
   spectateurs peuvent miser via `/pari`)
7. Au bout de 5 min, le combat se résout automatiquement via le moteur multi-rounds
8. Le résultat est posté dans le salon combat

### Cas particuliers

- **Refus** → Bob ne perd rien, toi tu récupères ta mise, pas de pénalité
- **Pas de réponse en 24 h** → combat expire, Bob perd **20 % de la mise en
  pénalité lâcheté** + son compteur `cowardice_count` augmente, toi tu
  récupères ta mise
- **`cowardice_count ≥ 5`** → Bob subit **-20 % sur tous ses gains de combat**
  (malus de lâcheté)
- **Attaque Surprise** (item `surprise`) → Bob **ne peut pas refuser**, le
  combat se résout instantanément
- **Explosion** (item `explosion`, côté défenseur) → combat annulé
  immédiatement, **les deux joueurs perdent 50 % de la mise**
- **Bloodbath** (event serveur) → Bob est **forcé d'accepter** et le combat
  démarre instantanément

### Matchmaking et handicap de niveau

| Écart de niveau | Malus ATK du plus fort | Bloqué ? |
|---|---|---|
| 0 – 2 | 1.0 (aucun malus) | ❌ |
| 3 – 5 | 0.8 (-20 %) | ❌ |
| 6 – 9 | 0.6 (-40 %) | ❌ |
| ≥ 10 | — | ✅ **interdit** |

---

## 🥊 Le moteur de combat (comment les HP baissent)

Un combat **Coup de Coude** se déroule en **3 à 7 rounds** selon les HP
combinés des deux joueurs :

- Combined HP < 250 → **3 rounds max**
- 250 ≤ Combined HP ≤ 400 → **5 rounds max**
- Combined HP > 400 → **7 rounds max**

### Round = 1 échange simultané

À chaque round :
1. **Chaque joueur jette un d20** (1-20)
2. Dégâts infligés = `(d20 × ATK_effective) ÷ 10`, puis réduction
   `dégâts × (1 - DEF / (DEF + 50))`, minimum **3 HP**
3. Les passifs s'appliquent :
   - **Bourrin** en berserker si HP ≤ 30 % → +25 % ATK ce round
   - **Agile** peut esquiver (15 %) → dégâts annulés
   - **Tank** → -5 flat sur les dégâts reçus
   - **Fourbe** → soigne 10 % des dégâts infligés
4. Le **chaos** peut frapper (8 % de chance par round, voir section Chaos)
5. Les deux joueurs perdent simultanément leurs HP
6. Si l'un tombe à 0 → **KO immédiat**, l'autre gagne

### Fin de combat

- **KO d'un des deux** → le survivant gagne
- **Fin des rounds sans KO** → le joueur avec le plus haut **% HP** gagne
- **Match nul exact** → aucun gagnant, personne ne perd de coins (sauf
  `accident_debile` chaos, voir section Chaos)

### Calcul des gains

Le gagnant récupère un pourcentage de la mise selon la marge de victoire :

| Écart de HP % | Gagnant reçoit | Perdant lâche |
|---|---|---|
| < 15 % | 70 % de la mise | 60 % de la mise |
| 15 – 40 % | 85 % de la mise | 80 % de la mise |
| > 40 % | 100 % de la mise | 100 % de la mise |

**Modificateurs** :
- **Fourbe gagnant** → +20 % de la mise en bonus
- **`cowardice_count ≥ 5`** → ×0.8 sur les gains
- **Happy Hour** actif → ×2 sur tous les gains
- **Giant Killer** → +15 XP bonus (pas de coins en plus)

Tous les gains sont **cappés sur le solde réel** du perdant (pas de création
de coins ex-nihilo).

---

## 🎲 Les événements chaos (Russian roulette)

À **chaque round**, il y a **8 % de chance** qu'un événement chaos frappe
aléatoirement :

| Event | Proba | Effet |
|---|---|---|
| 💥 **Critique Sauvage** | 2.0 % | L'attaquant inflige **x2 dégâts** ce round |
| ✨ **Esquive Divine** | 2.0 % | Le défenseur esquive **ET contre-attaque à +50 %** |
| 💩 **Accident Débile** | 1.5 % | Les deux joueurs prennent **10 % de leurs HP max** en auto-dégâts |
| 🩴 **Glissade** | 1.0 % | L'attaquant se **frappe lui-même** (dégâts sur soi) |
| 💰 **Vol à la Tire** | 1.5 % | Le gagnant du round **vole 5 % de la mise** en coins bonus (capé sur solde perdant) |

Sur un combat de 5 rounds, il y a **~34 % de chance** qu'un event chaos
tombe au moins une fois. Certains events sont bénéfiques, d'autres
catastrophiques — c'est ce qui donne au jeu son côté "chaotique".

---

## 🏪 Le shop (`/shop attaque | defense | braquage`)

Le shop est divisé en **3 sous-commandes** par catégorie. Chacune
affiche la liste filtrée des items et accepte un argument optionnel
`acheter:<item>` pour acheter directement.

- **`/shop attaque`** — items offensifs (rage, mindgame, double coup,
  poison, surprise, coup traître)
- **`/shop defense`** — potions, antidote, bouclier, explosion (qui
  est une carte defender-only)
- **`/shop braquage`** — outils consommables pour `/braquage` (voir la
  section dédiée)

Chaque catégorie affiche aussi ton inventaire filtré pour que tu voies
ce que tu as déjà.

### Items offensifs (consommés à l'usage en combat)

| Item | Prix | Effet |
|---|---|---|
| **Rage** 😡 | 100 | +50 % ATK, -30 % DEF pendant le combat (high risk) |
| **Mindgame** 🧠 | 150 | Révèle la classe et les HP adverses avant combat |
| **Explosion** 💣 | 200 | Les 2 joueurs perdent 50 % de la mise (annule le combat) |
| **Double Coup** 👊👊 | 250 | Lance 2d20 par round, garde le meilleur |
| **Poison** ☠️ | 300 | L'adversaire perd 5 HP par round |
| **Attaque Surprise** 💨 | 300 | L'adversaire **ne peut pas refuser** |
| **Coup Traître** 🗡️ | 350 | Réduit la DEF adverse de 50 % |

### Items défensifs

| Item | Prix | Effet |
|---|---|---|
| **Antidote** 💚 | 150 | Immunise contre le poison pendant 1 combat |
| **Bouclier** 🛡️ | 250 | +20 % DEF pendant tout le combat |

### Potions (utilisables hors combat)

| Item | Prix | Effet |
|---|---|---|
| **Potion de Soin** 🧪 | 80 | +30 HP (utiliser via `/potion`) |
| **Potion Majeure** 💊 | 200 | +80 HP (utiliser via `/potion`) |

### Items anti-vol — **déplacés vers `/protection`** (Phase 9)

Les anciens items anti-vol (Chien, Caméra, Coffre-fort) ne sont plus
vendus au shop : ils sont devenus des **abonnements secrets** souscrits
via `/protection`, avec 5 nouveaux items en plus. Voir la section
[Protections anti-vol](#️-protections-anti-vol-protection) ci-dessous.

---

## 🛡️ Les assurances (`/assurance`)

Avant d'entrer en combat risqué, tu peux souscrire une assurance qui
**réduit tes pertes de combat de 50 %**.

**Durées disponibles** :
- **1 jour** (1× prix de base)
- **1 semaine** (6× prix de base)
- **1 mois** (22× prix de base)

**Mais attention — il y a un % de chance que l'assurance soit une arnaque
(scam)** configurable par serveur (par défaut ~10 %). Dans ce cas :
- 💀 **Double les pertes** au lieu de les réduire
- Le message final révèle qu'elle était arnaqueuse

L'assurance est **automatiquement consommée** au premier combat perdu et
tu ne peux avoir **qu'une seule assurance active à la fois**.

> 💡 **Cumulable avec `/protection`** : l'assurance couvre tes pertes en
> combat, la protection bloque les tentatives de vol. Les deux tournent
> en parallèle sans interférer.

---

## 🛡️ Protections anti-vol (`/protection`)

Les anciens items anti-vol (chien/caméra/coffre) sont devenus des
**abonnements secrets** souscrits via `/protection`. Avantages :

- **Invisible aux voleurs** : la réponse de la commande est **ephemeral**
  (seul toi la vois). Personne ne peut inspecter ton inventaire pour
  deviner si tu es protégé.
- **Temps-base, pas de consommation** : tant que l'abonnement est actif,
  chaque tentative de vol déclenche un roll de blocage sans décompter
  de charge.
- **Cumulable** : tu peux avoir plusieurs items actifs en parallèle. À
  chaque vol subi, ils rollent dans l'ordre décroissant de bloc chance,
  premier qui réussit stoppe le vol.

### Syntaxe

```
/protection item:<item> duree:<1d|3d|5d|7d>
```

### Catalogue complet (8 items)

| Item | Block | Prix/jour |
|---|---|---|
| 🐕 **Chien de garde** | 25 % | 50 c |
| 🔔 **Alarme sonore** | 30 % | 80 c |
| 🪤 **Piège à loup** | 35 % | 120 c |
| 📹 **Caméra de surveillance** | 40 % | 160 c |
| 🍯 **Leurre doré** | 45 % | 220 c |
| 👮 **Garde du corps** | 50 % | 300 c |
| 🔒 **Coffre-fort** | 60 % | 450 c |
| 🏰 **Forteresse privée** | 70 % | 700 c |

### Grille de remise sur la durée

| Durée | Multiplicateur | Exemple (Chien 50 c/j) |
|---|---|---|
| 1 jour | × 1.0 | 50 c |
| 3 jours | × 2.7 (−10 %) | 135 c |
| 5 jours | × 4.25 (−15 %) | 213 c |
| 7 jours | × 5.6 (−20 %) | 280 c |

> Si tu souscris le même item pendant qu'un autre est encore actif, la
> nouvelle durée **s'ajoute** à l'expiration existante (cumul linéaire).

---

## 🗡️ Boost voleur (`/boost-voleur`)

Symétrique de `/protection` mais pour l'attaquant : souscris à un ou
plusieurs items qui ajoutent un **bonus flat au roll d20** du voleur
pendant la durée de l'abonnement. **Ephemeral** à l'activation — ta
cible ne saura pas que tu arrives boosté.

### Syntaxe

```
/boost-voleur item:<item> duree:<1d|3d|5d|7d>
```

### Catalogue (5 items, cumulatifs)

| Item | Bonus roll | Prix/jour |
|---|---|---|
| 🔧 **Crochet** | +5 | 60 c |
| 🗝️ **Passe-partout** | +10 | 120 c |
| 🥸 **Déguisement** | +15 | 200 c |
| 💨 **Fumigène** | +20 | 320 c |
| 🪚 **Marteau** | +25 | 500 c |

Les bonus **s'additionnent** : Crochet + Marteau actifs en parallèle =
+30 au roll. Utilise la même grille de remise 1/3/5/7 j que les
protections.

> Cumulatif avec le bonus de classe **Fourbe** (+4 au roll). Invisible
> dans l'affichage du combat si le boost est à 0, affiché séparément
> sinon pour que tu voies l'effet.

---

## 🎰 La cagnotte communautaire (`/cagnotte`)

Avant la Phase 9, les coins dépensés au shop, en assurance, en
protections, en taxe de `/donner`, en pénalité lâcheté ou en reset stats
**disparaissaient de l'économie** — inflation négative classique.

Maintenant ils tombent tous dans une **caisse communautaire par guild**
qui est **redistribuée chaque semaine** aux joueurs actifs (ceux qui ont
joué au moins un combat ou un vol dans les 7 derniers jours).

### Comment ça marche

- **Dépôts automatiques** : à chaque fois que tu paies quelque chose au
  jeu, la totalité (ou la taxe pour `/donner`) va dans la caisse.
- **Redistribution hebdo** : un worker déclenche la redistribution tous
  les 7 jours. Jusqu'à **20 gagnants aléatoires** par redistribution,
  avec des gains **disparates** (effet loterie : un gros gagnant qui
  empoche ~40-50 %, puis des petites mains).
- **Zéro action de ta part** : les coins arrivent directement sur ton
  wallet, ils sont loggés dans ton historique avec la source
  `coude_cashbox_redist`.

### Commande

- **`/cagnotte`** — affiche l'état actuel de la caisse (solde, total
  collecté depuis toujours, total déjà redistribué, date de la dernière
  redistribution)

> 💡 **Tu contribues en dépensant**, tu gagnes en **jouant**. Les
> joueurs qui lurkent sans risquer un combat ne touchent rien.

---

## 🎭 Le braquage (`/braquage`)

**Phase 10.** Une fois par semaine, tu peux tenter le **gros coup** sur la
caisse communautaire (la même cagnotte que `/cagnotte`). Très peu de
chances de réussir sans préparation, mais la récompense peut être
énorme.

### Principe

```
/braquage
```

- **Taux de base** : 5 %
- **Chaque outil consommable** dans ton inventaire ajoute **+5 %** au roll
- **Cap maximum** : 50 % (avec les 9 outils activés)
- **Cooldown** : 1 fois par **7 jours** par joueur
- Les **outils sont consommés** quel que soit le résultat (succès ou échec)

### Succès

Tu empoches **30 à 75 %** aléatoire du solde courant de la caisse →
crédit direct sur ton wallet. Un message doré est posté dans le salon
activités.

### Échec 💀

Tu es envoyé en **PRISON pour 24 heures**. Pendant ce temps **aucune
commande gameplay n'est utilisable** :

- ❌ `/coude`, `/voler`, `/pari`, `/prime`, `/braquage`
- ❌ `/shop`, `/potion`, `/protection`, `/boost-voleur`
- ❌ `/train`, `/classe`, `/donner`, `/repos`, `/reset-stats`

Seules les commandes passives restent accessibles : `/profil`,
`/cagnotte`, `/leaderboard`, `/hp`, `/saison`, `/resume`,
`/refuser`, `/annuler`.

### Les outils de braquage (9 items consommables)

Achetables via **`/shop braquage acheter:<item>`** :

| Outil | Bonus | Prix |
|---|---|---|
| 🎭 **Masque de braquage** | +5 % | 100 coins |
| 🔨 **Pied-de-biche** | +5 % | 150 coins |
| 🔓 **Crochet de vault** | +5 % | 220 coins |
| 🗺️ **Plan du coffre** | +5 % | 320 coins |
| 💨 **Fumigène de diversion** | +5 % | 450 coins |
| 💣 **Explosif** | +5 % | 600 coins |
| 💾 **Hacker kit** | +5 % | 800 coins |
| 🚁 **Drone espion** | +5 % | 1000 coins |
| 👪 **Équipe de pros** | +5 % | 1500 coins |

> Les 9 outils + les 5 % de base = **50 %** max. Les doublons ne
> comptent qu'une fois (achète chaque outil UNE fois pour maximiser).

### Stratégie

- Le **ROI est quasi-garanti** si tu achètes les 9 outils et que la
  caisse est grosse : 50 % de chance × 30-75 % d'une grosse caisse
  donne une espérance très rentable sur le long terme.
- **Mais** : un échec = 24 h sans jouer + tous les outils perdus. Si
  tu comptais sur la saison en cours, ça peut te coûter cher.
- Conseil : attends que la caisse soit **grosse** (`/cagnotte` pour
  vérifier) avant de tenter, pour maximiser le gain si tu réussis.

---

## 🔥 Les railleries automatiques

Le jeu track tes séries de **victoires**, **défaites** et **vols
subis**. Quand tu atteins un palier (**3, 5 ou 10**), un message
moqueur est posté dans un salon dédié (configuré par l'admin) **et**
ton pseudo Discord est renommé avec un suffixe progressif.

### Les paliers

| Palier | Victoires | Défaites | Vols subis |
|---|---|---|---|
| **3** | « en feu » | « (KO) » | « (vidé) » |
| **5** | « (tyran) » | « le Pouf » | « le Pigeon » |
| **10** | « le Légende » | « le Paillasson » | « la Tirelire » |

- Le **compteur se reset** dès que tu inverses la série (défaite après
  victoires, victoire après défaites, vol bloqué après vols subis).
- Un **match nul** reset les deux compteurs de combat.
- Les messages sont tirés d'un catalogue aléatoire par palier (3
  variantes par type × paliers).

### Opt-out

Si tu ne veux pas être raillé :

```
/no-taunts etat:on
```

Tu restes dans le jeu, tes streaks sont toujours trackées, mais tu
n'apparais **plus jamais** dans les messages railleurs et ton pseudo
n'est **pas renommé**. Pour re-rejoindre la fête : `/no-taunts etat:off`.

> Les admins peuvent forcer le retrait d'un opt-out via la page web
> `/coude/taunts`, mais le joueur peut retaper `/no-taunts on` à tout
> moment pour se re-protéger.

---

## 🎰 Les paris (`/pari`)

Pendant la **phase de paris** (5 min après que le défenseur accepte), les
spectateurs peuvent miser sur le vainqueur.

**`/pari combat:<id> sur:@user amount:50`**

Système **pari-mutuel avec commission 15 %** :
- **85 %** du pot total est redistribué aux parieurs qui ont backé le gagnant
  (proportionnel à leur mise)
- **10 %** va au **combattant gagnant** comme bonus
- **5 %** va au **combattant perdant** (récompense de participation)

**Égalité ou explosion** → tous les paris sont **entièrement remboursés**.

Règles :
- Impossible de parier sur son propre combat
- Impossible de parier après la fermeture de la fenêtre (protection race)
- Si tu parieras pendant que le worker résout → la commande est bloquée
  automatiquement avec un message d'erreur

---

## 🥷 Le vol (`/voler`)

**`/voler cible:@user`** tente de voler des coins à un autre joueur.

### Flow

1. Le voleur lance la commande
2. La cible reçoit une **alerte avec un bouton "Se défendre !"** (60 s)
3. Deux cas :
   - **La cible clique "Se défendre"** → **duel de rolls** (d20 + bonus)
   - **La cible ne clique pas (AFK)** → roll automatique **avec un malus
     de -8** sur le roll défenseur
4. Les items anti-vol de la cible peuvent **bloquer le vol** après le roll :
   - Coffre-fort (60 %) → Caméra (40 %) → Chien (25 %)
   - L'item est **consommé** s'il bloque
5. **Résolution** :
   - **Voleur gagne** → vole 10–25 % des coins de la cible (10-15 % si AFK)
   - **Cible gagne** → voleur perd **15 %** de ses propres coins
   - **Cible bloquée par item** → aucun échange, cible gagne +3 XP

**Limite quotidienne** : `steal_max_daily` tentatives par joueur par jour
(configurable par serveur, typiquement 3-5).

---

## 🎁 Les primes (`/prime`)

**`/prime @cible montant:500`** place une prime sur la tête d'un joueur.
Le prochain qui **bat la cible en combat** (ou qui tente un vol réussi, selon
config) **récupère la prime** en plus du gain normal.

- Les primes peuvent **s'empiler** — plusieurs joueurs peuvent déposer une
  prime sur la même cible
- **`/prime liste`** affiche les primes actives dans la guilde

---

## 💝 Dons (`/donner`)

**`/donner cible:@user don:coins montant:100`** — transfère des coins.

- Minimum : **10 coins**
- Taxe : **10 %** (le destinataire reçoit 90 %)
- Minimum de solde restant pour le donneur : **50 coins**

**`/donner cible:@user don:<item_key>`** — transfère un objet d'inventaire
(pas de taxe sur les items).

---

## 📊 Profil & stats

- **`/profil [user:@user]`** — affiche le profil complet (niveau, titre, HP,
  ATK/DEF effectives, XP, inventaire, assurance active)
- **`/leaderboard categorie:<richest|thieves|cowards|chaos|level>`** —
  classements du serveur
- **`/resume`** — ton résumé perso condensé
- **`/saison`** — infos sur la saison en cours (90 jours par défaut)

---

## 🔧 Autres commandes

- **`/accepter`** — accepte un défi en attente (alternative au bouton)
- **`/refuser`** — refuse un défi en attente
- **`/annuler`** — annule ton propre défi (si encore en `pending`)

---

## 💡 Stratégies & conseils

### Pour débutants

1. Commence **Tank** ou **Agile** — ta durée de vie sera meilleure
2. Vise les petites mises (10-30 coins) pour apprendre le moteur
3. Achète une **Potion de Soin** très tôt pour éviter d'attendre la régen
4. Évite les Bourrins tant que tu n'as pas d'ATK élevé
5. Parie sur les combats des autres pour gagner sans risque physique

### Pour joueurs intermédiaires

1. Monte **Fourbe** au niveau 10+ pour maximiser les gains
2. Utilise **Rage** contre un Tank si tu es Bourrin (high-risk/high-reward)
3. Pose des primes sur ceux qui t'ont battu — double vengeance
4. **Soucris une assurance** avant un gros combat (mais accepte le risque
   d'arnaque)

### Pour joueurs avancés

1. Exploite les handicaps : attaquer un niveau -3 te donne bonus +30 XP
2. Accumule des items anti-vol si tu as beaucoup de coins stockés
3. Surveille les events serveur (happy hour → tout × 2)
4. Gère ton `cowardice_count` — refuser trop de combats te coûte -20 % sur tes futurs gains

### Les pièges à éviter

- ❌ Accepter un combat contre un Fourbe niveau 15 quand tu es niveau 5 — handicap 0.6 mais risque de Giant Killer inversé
- ❌ Parier sur soi-même (interdit)
- ❌ Laisser ton HP descendre sous 10 % avant un combat (refus automatique au préconfirm)
- ❌ Spammer `/voler` sans items anti-vol à la maison

---

## ❓ FAQ

**Q : Est-ce que je perds mes stats quand je perds un combat ?**
A : Non, tu ne perds que des coins et des HP. Tes ATK/DEF/niveau/XP restent.

**Q : À quelle vitesse régénèrent mes HP ?**
A : Environ 10 à 100 HP/h selon ton % de vie actuel. Plus tu es bas, plus tu
régen vite. Skip pendant les combats en cours.

**Q : Puis-je avoir plusieurs combats en cours ?**
A : Non, **un seul combat en attente** (`pending`) ou en résolution à la fois.

**Q : Un défi peut-il expirer ?**
A : Oui, après **24 h** sans réponse. Le défenseur subit la pénalité lâcheté
(20 % de la mise + compteur de lâcheté).

**Q : Si je change de classe je perds mon niveau ?**
A : Non, tu gardes XP et level. Seuls ATK/DEF base changent (mais les points
que tu as dépensés avec `/train` restent).

**Q : C'est quoi la saison ?**
A : Tous les 90 jours, les stats de saison se réinitialisent (les wins, coins
gagnés, etc.). Ton level et ton wallet ne sont pas touchés.

**Q : Puis-je récupérer ma mise si le défenseur refuse ?**
A : Oui, intégralement. Pas de perte sur refus.

---

## 🎮 Commandes — récap

### Joueur — gameplay

| Commande | Description |
|---|---|
| `/profil [user:@user]` | Affiche ton profil (ou celui d'un autre) |
| `/classe` | Choisis/change ta classe |
| `/shop [acheter:<item>]` | Boutique (liste ou achat) |
| `/coude cible:@u mise:N [special]` | Défie un joueur |
| `/accepter` | Accepte un défi en attente |
| `/refuser` | Refuse un défi |
| `/annuler` | Annule ton propre défi |
| `/pari combat:id sur:@u amount:N` | Parie sur un combat |
| `/potion type:<soin\|majeure>` | Utilise une potion |
| `/repos` | Restauration complète HP (cd 12 h) |
| `/hp` | Affiche ta barre de vie |
| `/train stat:<attaque\|defense>` | Dépense 1 point de stats |
| `/reset-stats` | Redistribue tous tes points (300 coins) |
| `/voler cible:@u` | Tente de voler un joueur |
| `/assurance duree:<jour\|semaine\|mois>` | Souscris une assurance (perte combat -50 %) |
| `/prime cible:@u montant:N` | Pose une prime |
| `/donner cible:@u don:<type> montant:N` | Donne coins ou items |
| `/leaderboard categorie:<...>` | Classements |
| `/resume` | Résumé perso |
| `/saison` | Infos saison en cours |

### Joueur — Phase 9 (abonnements + caisse + railleries)

| Commande | Description |
|---|---|
| `/protection item:<…> duree:<1d\|3d\|5d\|7d>` | Abonnement anti-vol (ephemeral) |
| `/boost-voleur item:<…> duree:<1d\|3d\|5d\|7d>` | Abonnement boost du roll vol (ephemeral) |
| `/cagnotte` | État de la caisse communautaire + date redistribution |
| `/no-taunts etat:<on\|off>` | Opt-in/out des railleries automatiques te concernant |

### Joueur — Phase 10 (braquage)

| Commande | Description |
|---|---|
| `/braquage` | Tente le gros coup sur la caisse (1×/sem, base 5 %, prison 24 h sur échec) |
| `/shop braquage [acheter:<item>]` | Achat des 9 outils consommables qui boostent le roll |

### Admin

| Commande | Description |
|---|---|
| `/taunts-channel salon:<#…>` | Configure le salon des railleries (admin) |

Une page web admin `/coude/taunts` complète la commande : toggle
global, liste des opt-outs avec retrait forcé, picker de salon. Voir
la doc technique.

---

**Bon combat, et que les rolls te soient favorables ! ⚔️**

*Dernière mise à jour : 15 avril 2026 — Phase 9 (caisse + protections
+ boosts + railleries) et Phase 10 (braquage hebdomadaire + prison +
9 outils consommables).*
