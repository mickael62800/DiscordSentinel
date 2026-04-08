# REFONTE COMPLETE — Coup de Coude v2

> Document de conception pour la refonte totale du jeu Coup de Coude.
> Base sur l'audit d'equilibrage realise sur le code existant.

---

## Table des matieres

1. [Resume des problemes actuels](#1-resume-des-problemes-actuels)
2. [Systeme de combat v2 — Multi-rounds avec HP](#2-systeme-de-combat-v2--multi-rounds-avec-hp)
3. [Classes — Rework et activation](#3-classes--rework-et-activation)
4. [Formule de degats v2](#4-formule-de-degats-v2)
5. [Salons de combat temporaires](#5-salons-de-combat-temporaires)
6. [Systeme de surenchere](#6-systeme-de-surenchere)
7. [Shop v2 — Items, potions et dons](#7-shop-v2--items-potions-et-dons)
8. [Vol interactif](#8-vol-interactif)
9. [Casino v2 — Roue et Blackjack](#9-casino-v2--roue-et-blackjack)
10. [Chaos v2 — Evenements reworkes](#10-chaos-v2--evenements-reworkes)
11. [Progression et economie](#11-progression-et-economie)
12. [Reset saisonnier](#12-reset-saisonnier)
13. [Messages fun et flavour texts](#13-messages-fun-et-flavour-texts)
14. [Corrections de bugs existants](#14-corrections-de-bugs-existants)
15. [Nouvelles commandes](#15-nouvelles-commandes)
16. [Impact sur la base de donnees](#16-impact-sur-la-base-de-donnees)

---

## 1. Resume des problemes actuels

### Code mort
| Element | Probleme |
|---------|----------|
| 4 classes (Agile, Fourbe, Tank) | Tout le monde est Bourrin par defaut, aucune commande `/classe` |
| Esquive 15% (Agile) | Jamais declenchee |
| Steal bonus +20% (Fourbe) | Jamais applique |
| Croissances differenciees | Inutiles — meme classe pour tous |
| HP affiches (`100 + DEF/2`) | Purement cosmetique, jamais utilise en combat |
| Mindgame (150 coins) | Le joueur voit le jet adverse mais ne peut rien faire — 1 seul tour |
| `update_player_class()` dans db.rs | Fonction existante jamais appelee |

### Sureevalue (trop puissant)
| Element | Probleme |
|---------|----------|
| Coup Traitre (350 coins) | Ignore TOUTE la DEF adverse — victoire quasi-garantie |
| Inversion (500 coins) | Swap de coins apres le combat, timing casse |
| Attaque Surprise (300 coins) | Aucune contre-mesure possible |
| Critique Sauvage (5% chaos) | x3 stackable avec Giant Killer (x2) et Happy Hour (x2) = x12 |

### Sous-evalue (trop faible)
| Element | Probleme |
|---------|----------|
| La DEF | Soustractive dans une formule multiplicative — ne scale pas |
| Le Tank | Pas de passif, fait 5 degats quasi-systematiquement |
| XP de defaite (+5) | 3x moins que victoire, les faibles progressent lentement |
| Explosion (200 coins) | Le defenseur paie 200 + sa mise pour punir l'adversaire de sa mise |
| Assurance (50 coins) | Tres situationnel pour 1h |
| Vol (/voler) | Esperance negative si la cible a autant de coins que toi |

### Bugs de design
| Element | Probleme |
|---------|----------|
| Rage | Shop dit "+50 ATK -50 DEF", le code fait seulement +50 ATK |
| Inversion | Swap apres combat — ordre des operations imprevisible |
| Casino XP | `roll > 92` donne +10 XP au lieu de seulement le jackpot |
| Perdant | Perd toujours 100% de la mise meme si defaite serree |
| Giant Killer XP | 30 x 2 = 60 XP (4x normal) + handicap favorable = trop genereux |

---

## 2. Systeme de combat v2 — Multi-rounds avec HP

### Concept
Le combat passe d'un **jet unique** a un **combat en rounds** avec de vrais points de vie. Chaque round, les deux joueurs attaquent simultanement. Le premier a tomber a 0 HP perd.

### HP
```
HP_max = 100 + (DEF_effective x 2)
```
- Bourrin niv 1 : 100 + 16 = **116 HP**
- Agile niv 1 : 100 + 36 = **136 HP**
- Fourbe niv 1 : 100 + 28 = **128 HP**
- Tank niv 1 : 100 + 50 = **150 HP**

> La fonction `display_hp()` dans `progression.rs` existe deja — on l'adapte.

### Les HP sont persistants
- Les HP **ne se regenerent pas** automatiquement apres un combat
- Regeneration naturelle : **+10 HP par heure** (meme hors-ligne)
- Potions de soin achetables au shop
- Commande `/repos` : full HP, cooldown 12 heures
- Un joueur avec **moins de 20% de ses HP max** ne peut pas combattre

### Deroulement d'un combat

```
1. /coude @cible [mise] [item]
   L'attaquant defie quelqu'un

2. Le defenseur voit les boutons :
   [Accepter] [Objet] [Surencherir] [Refuser]

3. Phase de surenchere (optionnelle, voir section 6)

4. Le bot cree un salon temporaire : #⚔-darpone-vs-omelette

5. Phase de paris (configurable, defaut 5 min)
   Les spectateurs parient dans le salon du combat

6. Combat en rounds (3 a 7 rounds max selon les HP) :
   - Chaque round : les deux lancent 1d20
   - Degats calcules et appliques aux HP
   - Les passifs de classe se declenchent (et revelent la classe !)
   - Le bot affiche un message par round dans le salon

7. Fin du combat :
   - Premier a 0 HP → perd
   - Si max rounds atteint → celui avec le plus de HP% restant gagne
   - Match nul si meme % HP restant

8. Resolution : gains, XP, paris, primes

9. Le salon passe en lecture seule (24h) puis se supprime
```

### Nombre de rounds max
Le nombre de rounds max depend de la somme des HP des deux joueurs :
- Moins de 250 HP combines : **3 rounds**
- 250 a 400 HP : **5 rounds**
- Plus de 400 HP : **7 rounds**

Cela evite que deux Tanks se frappent pendant 20 rounds.

---

## 3. Classes — Rework et activation

### Principe
- La classe est **cachee** des autres joueurs
- Sur `/profil` : ta propre classe est visible, celle des autres affiche **"???"**
- En combat : les passifs se revelent **au moment ou ils se declenchent**
- Apres le combat : les classes des deux joueurs sont revelees dans le resume

### Les 4 classes (v2)

#### Bourrin (ATK: 25 | DEF: 8 | Croissance: +4/+1)
- **Passif — Berserker** : Quand ses HP tombent sous 30%, son ATK augmente de +25%
- **Identite** : Canon de verre. Frappe comme un camion mais s'ecroule vite
- **Revelation** : "La rage envahit [joueur]... Son attaque explose ! C'est un BOURRIN !"

#### Agile (ATK: 12 | DEF: 18 | Croissance: +2/+3)
- **Passif — Esquive** : 15% de chance d'esquiver completement une attaque par round (0 degats recus)
- **Identite** : Survivant. Difficile a toucher, finit par gagner a l'usure
- **Revelation** : "[joueur] fait un pas de cote et esquive completement le coup ! C'est un AGILE !"

#### Fourbe (ATK: 18 | DEF: 14 | Croissance: +3/+2)
- **Passif — Vampirisme** : Chaque attaque vole 10% des degats infliges en HP
- **Identite** : Sustain offensif. Se soigne en frappant
- **Revelation** : "[joueur] aspire l'energie de son adversaire ! C'est un FOURBE !"

#### Tank (ATK: 8 | DEF: 25 | Croissance: +1/+4)
- **Passif — Blindage** : Reduit de 5 tous les degats recus par round (flat, apres la formule)
- **Identite** : Mur. Quasiment impossible a tuer, gagne au temps
- **Revelation** : "Le coup rebondit sur [joueur] comme sur un mur ! C'est un TANK !"

### Choix et changement de classe

**Commande `/classe`** :
- Au premier lancement : menu de selection avec description de chaque classe (gratuit)
- Si le joueur a deja une classe : le changement coute **500 coins** + cooldown de **7 jours**
- Les points de stats (ATK/DEF manuels) sont **conserves** au changement (Option A)
- Le joueur peut faire un `/reset-stats` separe pour **300 coins** s'il veut redistribuer

**Pourquoi c'est cache ?** :
- Empeche le meta-gaming ("je sais qu'il est Tank, j'utilise Coup Traitre")
- Encourage la diversite de classes sur le serveur
- Cree un effet de surprise a chaque combat
- Le Mindgame (item shop) peut reveler la classe de l'adversaire avant le combat → valeur strategique enorme

### Code existant reutilise
- `classes.rs` : les 4 classes sont deja definies avec stats, croissance, dodge_chance, steal_bonus
- `db.rs:update_player_class()` : fonction deja implementee pour changer la classe
- `db.rs:Player.class` : champ existant en base
- Migration `060` : colonne `class TEXT NOT NULL DEFAULT 'bourrin'`

### A modifier
- Ajouter le champ `dodge_chance` → passif generique par classe
- Remplacer `steal_bonus` par un systeme de passif plus flexible
- Le `DEFAULT 'bourrin'` en base ne change pas — les nouveaux joueurs commencent Bourrin et choisissent ensuite

---

## 4. Formule de degats v2

### Probleme actuel
```
degats = max(5, (jet x ATK / 50) - DEF)
```
L'ATK est multiplicative, la DEF est soustractive. La DEF ne scale pas.

### Nouvelle formule
```
degats_bruts = (jet x ATK) / 10
reduction = DEF / (DEF + 50)
degats = max(3, degats_bruts x (1 - reduction))
```

**Pourquoi c'est mieux :**
- La DEF est maintenant un **pourcentage de reduction**, pas une soustraction
- 25 DEF = 33% de reduction → significatif mais pas ecrasant
- 50 DEF = 50% de reduction → fort mais pas invincible
- 100 DEF = 67% de reduction → plafond naturel, jamais 100%
- L'ATK et la DEF scalent toutes les deux de maniere pertinente

**Jet de des : 1d20** au lieu de 1d100 :
- Reduit la variance (le jet va de 1 a 20, pas 1 a 100)
- Sur plusieurs rounds, la loi des grands nombres lisse la chance
- Les stats comptent plus que la chance d'un seul jet

### Exemples concrets (niveau 1)

**Bourrin (25 ATK) vs Tank (25 DEF) — jet moyen de 10 :**
```
Bourrin attaque : (10 x 25) / 10 = 25 bruts
Tank reduit : 25 / (25+50) = 33% reduction
Degats : 25 x 0.67 = ~17 degats
```

**Tank (8 ATK) vs Bourrin (8 DEF) — jet moyen de 10 :**
```
Tank attaque : (10 x 8) / 10 = 8 bruts
Bourrin reduit : 8 / (8+50) = 14% reduction
Degats : 8 x 0.86 = ~7 degats
```

**Resultat** : Le Bourrin fait 17 degats, le Tank fait 7. Mais le Tank a 150 HP et le Bourrin 116 HP. Il faut ~7 rounds pour tuer le Tank, ~17 rounds pour tuer le Bourrin. En 5 rounds max : le Tank survit probablement avec plus de HP%. C'est **equilibre**.

### Plancher de degats
Le plancher passe de 5 a **3**. Suffisant pour eviter les combats a 0 degats, mais assez bas pour que la DEF ait un vrai impact.

---

## 5. Salons de combat temporaires

### Concept
Quand un combat est accepte (et la surenchere resolue), le bot **cree un salon textuel temporaire** dans une categorie dediee.

### Fonctionnement
```
Categorie : ⚔ Arene (configurable par le serveur)

Salon cree : #⚔-darpone-vs-omelette
Permissions :
  - Tout le monde peut lire
  - Seuls les combattants et le bot peuvent ecrire (pendant le combat)
  - Apres le combat : tout le monde peut ecrire (reactions, GG, etc.)
```

### Cycle de vie
1. **Creation** : a l'acceptation du combat (ou apres surenchere)
2. **Phase paris** : les spectateurs sont invites a parier (`/pari`) dans ce salon
3. **Combat** : un message par round, embeds avec barres de vie, degats, passifs
4. **Resultat** : embed final avec resume complet
5. **Post-combat** : le salon reste ouvert 1 heure pour les discussions
6. **Archivage** : le salon passe en lecture seule
7. **Suppression** : automatique apres 24 heures

### Affichage d'un round (exemple)
```
━━━ ROUND 2/5 ━━━

🎲 Darpone lance... 14 !
💥 Darpone inflige 22 degats a Omelette !

🎲 Omelette lance... 8 !
🛡️ Le coup rebondit sur Darpone comme sur un mur ! C'est un TANK !
💥 Omelette inflige 4 degats a Darpone ! (Blindage : -5)

❤️ Darpone : ████████░░ 126/150 HP
❤️ Omelette : █████░░░░░ 67/128 HP
```

### Configuration
- `channel_arena_category` : ID de la categorie ou creer les salons
- `combat_channel_ttl_hours` : duree avant suppression (defaut 24h)

### Permission necessaire
Le bot a besoin de `MANAGE_CHANNELS` dans la categorie Arene.

---

## 6. Systeme de surenchere

### Concept
Quand un joueur est defie, il peut **surencherir** sur la mise au lieu de simplement accepter.

### Flux
```
1. Attaquant : /coude @Omelette 50
   "Darpone defie Omelette pour 50 coins !"

2. Omelette voit les boutons :
   [Accepter 50] [Surencherir] [Objet] [Refuser]

3. Si Omelette clique "Surencherir" :
   → Le bot demande un montant (modal Discord ou boutons predefinis : x2, x3, x5, custom)
   → "Omelette relance a 150 coins !"

4. Darpone recoit une notification :
   [Accepter 150] [Surencherir] [Se coucher]

5. Le ping-pong continue (max 3 surencheres au total pour eviter l'infini)

6. "Se coucher" = abandonner avant le combat → perd 10% de la derniere mise proposee
   C'est une penalite pour avoir lance un defi sans avoir les moyens
```

### Regles
- Chaque surenchere doit etre **strictement superieure** a la mise precedente
- Maximum **3 surencheres** (mise initiale + 3 relances)
- Le joueur qui se couche perd **10% de la derniere mise** en penalite
- Un joueur ne peut pas surencherir au-dela de ses coins disponibles
- L'attaquant ne peut pas annuler gratuitement apres une surenchere du defenseur

### Impact strategique
- Un joueur riche peut intimider un joueur pauvre avec une surenchere massive
- Un joueur confiant dans sa classe (cachee !) peut relancer pour maximiser ses gains
- Le "se coucher" a un cout — ca evite les defis gratuits sans consequence

---

## 7. Shop v2 — Items, potions et dons

### Items de combat (reworkes)

| Item | Prix | Effet v2 | Changement |
|------|------|----------|------------|
| **Rage** | 100 | +50% ATK mais -30% DEF pendant tout le combat | Fix : le malus DEF est maintenant applique (bug corrige) |
| **Double Coup** | 250 | Lance 2d20 et garde le meilleur **a chaque round** | Buff : s'applique a tous les rounds, pas un seul |
| **Coup Traitre** | 350 | Reduit la DEF adverse de **50%** (pas 100%) | Nerf : ne supprime plus toute la DEF |
| **Attaque Surprise** | 300 | L'adversaire ne peut pas refuser MAIS ne peut pas surencherir. Pas de phase de paris. | Identique mais le defenseur garde son item defensif |
| **Explosion** | 200 | **Les deux joueurs** perdent 50% de la mise (au lieu de 100%) + le combat est annule | Buff : cout effectif reduit, plus viable comme item defensif |
| **Mindgame** | 150 | **Revele la classe** de l'adversaire + ses HP actuels avant le round 1 | Rework : utile maintenant que les classes sont cachees |
| **Inversion** | ~~500~~ | **RETIRE** — Le swap de coins etait un design casse. Remplace par Poison. | Supprime |

### Nouveaux items

| Item | Prix | Effet |
|------|------|-------|
| **Potion de soin** | 80 | +30 HP (utilisable hors combat) |
| **Potion majeure** | 200 | +80 HP (utilisable hors combat) |
| **Antidote** | 150 | Immunise contre le poison pendant 1 combat |
| **Poison** | 300 | L'adversaire perd 5 HP par round pendant le combat |
| **Bouclier** | 250 | +20% DEF pendant tout le combat |

### Systeme de don

**Commande `/donner @joueur [type] [quantite]`**

Types de dons :
- **Items** : `/donner @Omelette potion_soin 2` → transfert 2 potions de soin
- **Coins** : `/donner @Omelette coins 500` → transfert 500 coins avec **taxe de 10%** (Omelette recoit 450)

Regles :
- Pas de don a soi-meme
- Pas de don de coins si le donneur tombe sous 50 coins apres le don
- Taxe de 10% sur les coins pour eviter les transferts abusifs (gold sink)
- Pas de cooldown sur les dons d'items
- Cooldown de **1 heure** sur les dons de coins (eviter le farming)
- Les dons sont logues en base pour detecter les abus

### Regeneration naturelle des HP
- **+10 HP par heure** (meme hors-ligne), calcule au moment de l'action suivante
- Plafonne aux HP max du joueur
- Commande `/repos` : regenere **100% des HP**, cooldown de **12 heures**

---

## 8. Vol interactif

### Concept
Le vol devient un mini-jeu interactif ou la victime peut se defendre.

### Flux

```
1. Le voleur fait /voler @cible

2. Le bot envoie un message PUBLIC dans le salon activites :
   "⚠️ Quelqu'un tente de voler @Omelette !"
   (Le nom du voleur N'EST PAS revele)

   Bouton : [🛡️ Se defendre !] (seule la cible peut cliquer)
   Timer : 60 secondes

3a. Si la cible NE REAGIT PAS (60s ecoulees) :
    → Vol reussi automatiquement
    → Gain : 10-15% des coins de la cible
    → Le voleur est revele : "Darpone a subtilise 45 coins a Omelette pendant qu'il dormait !"

3b. Si la cible CLIQUE "Se defendre" :
    → Duel de des !
    → Voleur lance 1d20 + bonus Fourbe (+4 si classe Fourbe)
    → Victime lance 1d20 + bonus DEF (DEF_effective / 10)

    Si voleur > victime :
      → Vol reussi, gain 15-25% des coins de la cible
      → "Malgre la resistance, Darpone reussit a chiper 78 coins a Omelette !"

    Si victime >= voleur :
      → Vol echoue !
      → Le voleur perd 15% de SES coins
      → L'identite du voleur est REVELEE
      → La victime gagne +3 XP "vigilance"
      → "Omelette attrape la main de Darpone dans sa poche ! Vol dejoue !"
```

### Regles
- Cooldown : **30 minutes** entre deux tentatives de vol
- Minimum 10 coins sur la cible
- Pas de vol sur un joueur en combat
- Pas de vol sur un joueur qui a moins de 20% HP (il a deja assez de problemes)

### Messages fun (vol reussi sans reaction)
```rust
const STEAL_SUCCESS_AFK: &[&str] = &[
    "💰 {voleur} a fait les poches de {victime} pendant sa sieste ! (-{montant} coins)",
    "🕵️ {voleur} s'est glisse dans l'ombre et a chipe {montant} coins a {victime} !",
    "🎭 {voleur} a distrait {victime} avec un tour de magie et lui a pique {montant} coins !",
    "🐱 {voleur} a vole {montant} coins a {victime} avec l'agilite d'un chat !",
    "💤 {victime} dormait sur son tresor... {voleur} en a profite pour prendre {montant} coins !",
    "🧲 {voleur} a utilise un aimant a coins sur {victime}. {montant} coins aspires !",
    "🎪 Pendant que {victime} regardait ailleurs, {voleur} a embarque {montant} coins !",
];
```

### Messages fun (vol reussi avec resistance)
```rust
const STEAL_SUCCESS_FIGHT: &[&str] = &[
    "💪 {victime} s'est debattu, mais {voleur} est plus malin ! {montant} coins voles !",
    "🤼 Apres une lutte acharnee, {voleur} repart avec {montant} coins de {victime} !",
    "🏃 {voleur} a arrache le sac de {victime} et s'est enfui en courant ! {montant} coins !",
    "🎯 {victime} a tente de bloquer mais {voleur} a feinte ! {montant} coins en poche !",
];
```

### Messages fun (vol echoue)
```rust
const STEAL_FAIL: &[&str] = &[
    "🚨 {victime} a attrape {voleur} la main dans le sac ! {voleur} perd {montant} coins de honte !",
    "👊 {victime} a mis une gifle a {voleur} en pleine tentative ! -{montant} coins pour le voleur !",
    "🍌 {voleur} a glisse sur une peau de banane en essayant de voler {victime} ! -{montant} coins !",
    "🐕 Le chien de {victime} a mordu {voleur} ! Vol rate et {montant} coins en frais medicaux !",
    "🪤 {victime} avait pose un piege ! {voleur} se retrouve suspendu par les pieds ! -{montant} coins !",
    "👀 {voleur} pensait etre discret... {victime} le regardait depuis le debut ! -{montant} coins !",
    "🤡 {voleur} a essaye de pickpocket {victime} mais a sorti son propre portefeuille par erreur ! -{montant} coins !",
    "🦴 {voleur} s'est pris les pieds dans le tapis en approchant {victime} ! Honteux. -{montant} coins !",
];
```

---

## 9. Casino v2 — Roue et Blackjack

### Commande
`/casino [jeu] [mise]`
- `jeu` : choix entre **"roue"** et **"blackjack"** (obligatoire)
- `mise` : montant a miser

Les cooldowns et limites journalieres s'appliquent a l'ensemble des jeux de casino.

### La Roue (rework de l'existant)

Le concept reste le meme mais avec plus de paliers et un affichage anime.

**Nouveaux paliers (tirage sur 10000) :**

| Palier | Probabilite | Resultat |
|--------|-------------|----------|
| Faillite | 0.5% (50/10000) | Perd TOUS ses coins |
| Perte | 49.5% (4950/10000) | Perd la mise |
| x1 (neutre) | 20% (2000/10000) | Recupere sa mise (ni gain ni perte) |
| x1.5 | 15% (1500/10000) | Gain modeste |
| x2 | 10% (1000/10000) | Bon gain |
| x3 | 3% (300/10000) | Gros gain |
| x5 | 1.5% (150/10000) | Tres gros gain |
| x10 Jackpot | 0.05% (50/10000) | JACKPOT + 10 XP |

**Affichage anime** : le bot edite le message 3-4 fois avec des emojis qui "defilent" pour simuler la roue qui tourne, avant d'afficher le resultat final.

**Messages fun :**
```rust
const ROUE_FAILLITE: &[&str] = &[
    "💣 La roue s'arrete sur FAILLITE ! {joueur} perd TOUT ! {montant} coins partis en fumee !",
    "☠️ FAILLITE TOTALE ! La banque envoie ses hommes de main chez {joueur}. {montant} coins saisis !",
    "🕳️ {joueur} tombe dans le trou de la faillite ! Bye bye les {montant} coins !",
];

const ROUE_JACKPOT: &[&str] = &[
    "👑 JAAACKPOT x10 !!! {joueur} empoche {montant} coins ! LES MURS TREMBLENT !",
    "🎆 LA LEGENDE ! {joueur} decroche le JACKPOT x10 ! {montant} coins ! Le casino pleure !",
    "💎 {joueur} fait exploser la banque ! JACKPOT x10 ! {montant} coins ! HISTORIQUE !",
];
```

### Le Blackjack (nouveau)

Un vrai jeu de cartes interactif avec des choix strategiques.

**Regles :**
- Le bot distribue 2 cartes au joueur et 2 au croupier (une face cachee)
- Valeurs : 2-10 = valeur, Valet/Dame/Roi = 10, As = 1 ou 11
- Objectif : se rapprocher de 21 sans depasser

**Actions (boutons Discord) :**

| Bouton | Effet |
|--------|-------|
| **Tirer** | Recevoir une carte supplementaire |
| **Rester** | Garder sa main actuelle |
| **Doubler** | Doubler la mise + recevoir exactement 1 carte + rester |

**Regles du croupier :**
- Le croupier tire obligatoirement si < 17
- Le croupier reste si >= 17

**Resultats :**
| Situation | Resultat |
|-----------|----------|
| Joueur bust (> 21) | Perd la mise |
| Croupier bust | Joueur gagne x2 |
| Joueur > croupier | Joueur gagne x2 |
| Croupier > joueur | Joueur perd la mise |
| Egalite | Mise remboursee |
| Blackjack naturel (21 avec 2 cartes) | Joueur gagne x2.5 |

**Timeout :** 30 secondes par action. Si le joueur ne clique pas → "Rester" automatique.

**Affichage :**
```
🃏 BLACKJACK — Mise : 100 coins

Tes cartes : 🂡 🂫  (Total : 20)
Croupier :   🂢 🎴  (Total : 2 + ?)

[Tirer] [Rester] [Doubler]
```

Apres resolution :
```
🃏 BLACKJACK — Resultat

Tes cartes : 🂡 🂫  (Total : 20)
Croupier :   🂢 🂹  (Total : 11) → tire 🂧 (Total : 18)

🎉 Tu gagnes ! +200 coins !
```

**Messages fun Blackjack :**
```rust
const BJ_BUST: &[&str] = &[
    "💥 BUST ! {joueur} a ete trop gourmand ! {total} points... c'est la cata !",
    "🤦 {joueur} depasse 21 avec {total} ! Le croupier ricane.",
    "📈 {joueur} pensait que plus c'est haut mieux c'est... {total} points. Non.",
];

const BJ_NATURAL: &[&str] = &[
    "🌟 BLACKJACK NATUREL ! {joueur} sort 21 du premier coup ! Legendaire !",
    "✨ 21 en deux cartes ! {joueur} est un dieu du Blackjack ! x2.5 !",
];

const BJ_WIN: &[&str] = &[
    "😎 {joueur} l'emporte avec {total} contre {croupier} ! +{gain} coins !",
    "🃏 La main de maitre ! {joueur} bat le croupier {total} a {croupier} !",
];

const BJ_LOSE: &[&str] = &[
    "😤 Le croupier gagne avec {croupier} contre {total}. -{mise} coins.",
    "🎰 Pas de chance ! Le croupier avait {croupier}. {joueur} rage.",
];
```

---

## 10. Chaos v2 — Evenements reworkes

### Concept
Les evenements chaos se declenchent maintenant **pendant** un round specifique du combat (pas avant la resolution). Chaque round a une chance independante de declencher un evenement.

### Probabilite
**8% de chance par round** (au lieu de 18% pour tout le combat). Sur un combat de 5 rounds, la probabilite d'avoir au moins un evenement chaos est d'environ 34%.

### Evenements (reworkes)

| Evenement | Proba par round | Effet v2 |
|-----------|-----------------|----------|
| **Critique Sauvage** | 2% | Ce round, l'attaquant inflige **x2 degats** (pas x3 sur les gains) |
| **Esquive Divine** | 2% | Le defenseur esquive ce round ET contre-attaque avec +50% degats |
| **Accident Debile** | 1.5% | Les deux joueurs prennent **10% de leurs HP max** en degats (se cognent la tete) |
| **Glissade** | 1% | L'attaquant se frappe lui-meme ce round (ses degats lui sont appliques) |
| **Vol a la Tire** | 1.5% | Le gagnant de ce round vole **5% des coins** de l'adversaire (en plus des degats) |

### Changements cles
- Plus de Critique Sauvage x3 sur les gains finaux (trop explosif) → x2 degats sur un round
- Les chaos sont **locaux a un round**, pas globaux au combat
- Ca cree des retournements de situation mid-combat ("Round 4 : GLISSADE ! Darpone se frappe !")
- Les multiplicateurs ne se stackent plus de maniere explosive

### Messages fun chaos
```rust
const CHAOS_CRITIQUE: &[&str] = &[
    "💥 CRITIQUE SAUVAGE ! {attaquant} met TOUTE sa force dans ce coup !",
    "⚡ Un eclair de puissance ! {attaquant} frappe deux fois plus fort !",
    "🔥 {attaquant} voit rouge et declenche un coup DEVASTATEUR !",
];

const CHAOS_ESQUIVE: &[&str] = &[
    "✨ ESQUIVE DIVINE ! {defenseur} esquive avec grace et contre-attaque !",
    "🌀 {defenseur} disparait comme un ninja et frappe dans le dos !",
    "🪞 {defenseur} fait un pas de cote digne d'un film et riposte !",
];

const CHAOS_ACCIDENT: &[&str] = &[
    "💩 ACCIDENT DEBILE ! Les deux joueurs se cognent la tete en meme temps !",
    "🤡 Les deux glissent dans une flaque et se font mal !",
    "🐔 Un poulet traverse l'arene ! Les deux trebuchent !",
];

const CHAOS_GLISSADE: &[&str] = &[
    "🩴 GLISSADE ! {attaquant} marche sur une peau de banane et se frappe !",
    "🧊 {attaquant} glisse sur du verglas et s'auto-KO ce round !",
    "🤸 {attaquant} tente une pirouette... et se met un coup de coude a lui-meme !",
];

const CHAOS_VOL: &[&str] = &[
    "💰 VOL A LA TIRE ! Pendant le chaos, des coins tombent des poches !",
    "🐒 Un singe vole des coins et les donne au plus fort !",
    "🌪️ Le vent souffle des coins d'une poche a l'autre !",
];
```

---

## 11. Progression et economie

### XP (reworkee)

| Action | XP actuel | XP v2 | Raison |
|--------|-----------|-------|--------|
| Victoire combat | 15 | **20** | Recompense principale |
| Defaite combat | 5 | **10** | Monte : reduire l'ecart entre gagnants et perdants |
| Vol reussi | 5 | **5** | Identique |
| Defense vol reussie | 0 | **3** | Nouveau : recompense la vigilance |
| Jackpot casino (x10) | 10 | **10** | Identique (mais bug fixe : seulement le x10) |
| Blackjack naturel | - | **5** | Nouveau |
| Giant killer (bonus) | x2 | **+15 XP bonus** | Change : bonus fixe au lieu de multiplicateur (evite les 60 XP) |

### Niveau max et stat points
- **25 niveaux** (inchange)
- **3 points par niveau** (inchange)
- Points distribuables dans ATK ou DEF via `/train`

### Gold sinks (puits a coins)
Le probleme actuel : les coins circulent entre joueurs mais ne disparaissent jamais (sauf faillite casino). Nouveaux gold sinks :

| Depense | Cout | Frequence |
|---------|------|-----------|
| Potions de soin | 80-200 | Reguliere (apres chaque combat) |
| Changement de classe | 500 | Rare (1x par semaine max) |
| Reset de stats | 300 | Rare |
| Taxe sur les dons de coins | 10% du montant | A chaque don |
| Penalite "se coucher" (surenchere) | 10% de la mise | A chaque abandon |
| Poison, Antidote, Bouclier | 150-300 | Reguliere |

### Matchmaking handicap (inchange)
- 0-2 niveaux d'ecart : pas de handicap
- 3-5 : -20% ATK pour le plus fort
- 6-9 : -40% ATK pour le plus fort
- 10+ : combat bloque

### Gains de combat (reworkes)

**Gagnant :**
```
gain = mise x pourcentage_marge
```
- Marge serree (< 15% HP diff) : 70% de la mise
- Marge correcte (15-40%) : 85% de la mise
- Marge nette (> 40%) : 100% de la mise

**Perdant :**
```
perte = mise x pourcentage_marge_inverse
```
- Marge serree : perd 60% de la mise (au lieu de 100% actuellement)
- Marge correcte : perd 80%
- Marge nette : perd 100%

> Le perdant ne perd plus systematiquement 100%. Une defaite serree coute moins cher.

### Assurance (reworkee)
- Prix : **100 coins** (au lieu de 50)
- Duree : **3 heures** (au lieu de 1)
- Protection : -50% pertes de combat (inchange)
- Risque arnaque : **3%** (au lieu de 5%, reduction car plus cher)
- Effet arnaque : +50% pertes (au lieu de +100%, moins punitif)

---

## 12. Reset saisonnier

### Concept
Tous les **3 mois**, une saison se termine et le jeu est reset. Cela cree un cycle competitif avec un gagnant par saison.

### Ce qui est reset
- Coins → remis a **100** (valeur de depart)
- Niveau → remis a **1**
- XP → remis a **0**
- Stats manuelles (ATK/DEF) → remises a **0**
- Inventaire → vide
- HP → remis a HP max de base
- Wins/Losses/Draws → remis a 0
- Compteur lachetee → remis a 0

### Ce qui est conserve
- La **classe** choisie
- L'historique des combats (en base, pour les stats globales)
- Les **titres de saison** gagnes (voir ci-dessous)

### Fin de saison
Avant le reset, le bot annonce les **classements finaux** :

1. **Champion de la saison** : joueur avec le plus de coins → titre permanent "Champion S1" (ou S2, etc.)
2. **Meilleur combattant** : plus de victoires
3. **Plus gros voleur** : plus de coins voles
4. **Roi du chaos** : plus d'evenements chaos declenches
5. **Legende du casino** : plus gros gain unique au casino

Ces titres sont affiches sur le profil du joueur de maniere permanente (badge/emoji).

### Calendrier
- **Saison 1** : lancement → 3 mois apres
- Annonce 1 semaine avant la fin
- Annonce 24h avant le reset
- Reset automatique a minuit

---

## 13. Messages fun et flavour texts

### Combat — Debut
```rust
const COMBAT_START: &[&str] = &[
    "⚔️ {attaquant} craque ses doigts et regarde {defenseur} droit dans les yeux...",
    "🔔 DING DING ! Le match {attaquant} vs {defenseur} commence !",
    "🎬 Les lumieres s'eteignent... le spot s'allume sur {attaquant} et {defenseur} !",
    "🌪️ L'arene tremble ! {attaquant} et {defenseur} entrent en scene !",
    "☠️ Ca va saigner ! {attaquant} defie {defenseur} ! Prenez vos popcorns !",
];
```

### Combat — Round (attaque normale)
```rust
const ROUND_ATTACK: &[&str] = &[
    "💥 {attaquant} envoie un coup de coude VIOLENT ! {degats} degats !",
    "👊 {attaquant} frappe avec precision ! {degats} degats infliges !",
    "🦵 {attaquant} enchaine avec un coup vicieux ! {degats} degats !",
    "💫 {attaquant} met toute sa force dans ce coup ! {degats} degats !",
    "🥊 BOUM ! {attaquant} connecte un coup solide ! {degats} degats !",
    "🌟 {attaquant} surgit et frappe ! {degats} degats !",
    "😤 {attaquant} rugit et balance un coup enorme ! {degats} degats !",
];
```

### Combat — Round (degats faibles / bloque)
```rust
const ROUND_WEAK: &[&str] = &[
    "🛡️ {defenseur} encaisse le coup sans broncher ! Seulement {degats} degats.",
    "😴 {attaquant} tape comme un chatonnet... {degats} degats. Genant.",
    "🧱 {defenseur} est un MUR. {degats} petits degats.",
    "🪨 Le coup de {attaquant} rebondit sur {defenseur}. {degats} degats, c'est tout.",
    "🐜 {attaquant} chatouille {defenseur}. {degats} degats. Vraiment ?",
];
```

### Combat — KO
```rust
const COMBAT_KO: &[&str] = &[
    "☠️ {perdant} s'ecroule ! K.O. ! {gagnant} remporte le combat !",
    "💀 C'est TERMINE ! {perdant} est a terre ! {gagnant} leve le poing !",
    "🏆 {gagnant} acheve {perdant} avec un dernier coup ! VICTOIRE !",
    "🎤 *Et le nouveau champion est...* {gagnant} ! {perdant} peut ramasser ses dents.",
    "🪦 Repose en paix la dignite de {perdant}. {gagnant} domine !",
];
```

### Combat — Fin au temps (HP restants)
```rust
const COMBAT_TIMEOUT: &[&str] = &[
    "⏰ TEMPS ECOULE ! {gagnant} gagne aux points ({hp_g}% HP vs {hp_p}% HP) !",
    "🔔 Fin du match ! {gagnant} l'emporte avec {hp_g}% de vie restante !",
    "📊 Les juges tranchent : {gagnant} gagne avec {hp_g}% HP contre {hp_p}% pour {perdant} !",
];
```

### Combat — Match nul
```rust
const COMBAT_DRAW: &[&str] = &[
    "🤝 Les deux combattants sont a bout de souffle ! Match nul !",
    "⚖️ Impossible de les departager ! Egalite parfaite !",
    "🫠 Personne ne gagne... personne ne perd... c'est frustrant.",
];
```

### Surenchere
```rust
const SURENCHERE: &[&str] = &[
    "💰 {joueur} rigole et relance a {montant} coins ! \"T'as les moyens ?\"",
    "🎰 {joueur} jette {montant} coins sur la table ! \"On monte les encheres !\"",
    "😏 {joueur} surencherit a {montant} coins avec un sourire narquois.",
];

const SE_COUCHER: &[&str] = &[
    "🐔 {joueur} se couche... Pas les moyens ou pas le courage ? -{penalite} coins.",
    "🏳️ {joueur} abandonne la surenchere. La honte lui coute {penalite} coins.",
    "💨 {joueur} disparait dans la fumee... il laisse {penalite} coins sur la table.",
];
```

---

## 14. Corrections de bugs existants

| Bug | Fichier | Correction |
|-----|---------|------------|
| Rage : "+50 ATK -50 DEF" mais code fait seulement +50 ATK | `combat.rs:88` | Ajouter `def_bonus_flat -= 30` (v2 : -30% DEF) |
| Casino XP : `roll > 92` donne XP au x5 ET au jackpot | `casino.rs:222` | Changer condition pour uniquement le palier jackpot |
| Inversion : swap apres combat, timing incoherent | `accepter.rs:341-349` | Item supprime, remplace par Poison |
| Perdant perd 100% meme si defaite serree | `combat.rs:232` | Perte proportionnelle a la marge (voir section 11) |
| Classe par defaut sans choix | `db.rs:160` | Garder default 'bourrin', ajouter commande `/classe` |
| `display_hp()` cosmetique | `progression.rs:50` | Utiliser la formule pour les vrais HP de combat |

---

## 15. Nouvelles commandes

| Commande | Description |
|----------|-------------|
| `/classe` | Choisir ou changer sa classe (gratuit 1ere fois, 500 coins ensuite, cooldown 7j) |
| `/reset-stats` | Redistribuer tous ses points de stats (300 coins) |
| `/donner @joueur [type] [quantite]` | Donner des items ou des coins a un autre joueur |
| `/repos` | Regenerer tous ses HP (cooldown 12h) |
| `/hp` | Voir ses HP actuels et le timer de regeneration |
| `/casino roue [mise]` | Jouer a la roue |
| `/casino blackjack [mise]` | Jouer au blackjack |
| `/saison` | Voir le temps restant et les classements de la saison en cours |

### Commandes modifiees

| Commande | Changement |
|----------|------------|
| `/coude` | Nouveau flux avec surenchere + creation de salon |
| `/voler` | Devient interactif avec alerte et defense |
| `/profil @joueur` | Classe cachee si c'est un autre joueur + affiche HP |
| `/shop` | Nouveaux items (potions, poison, antidote, bouclier) |
| `/assurance` | Prix et duree ajustes |
| `/leaderboard` | Ajout classement saisonnier |

---

## 16. Impact sur la base de donnees

### Colonnes a ajouter sur `coude_players`
```sql
ALTER TABLE coude_players ADD COLUMN hp_current INTEGER NOT NULL DEFAULT 100;
ALTER TABLE coude_players ADD COLUMN hp_max INTEGER NOT NULL DEFAULT 100;
ALTER TABLE coude_players ADD COLUMN hp_last_regen TIMESTAMPTZ NOT NULL DEFAULT NOW();
ALTER TABLE coude_players ADD COLUMN class_changed_at TIMESTAMPTZ;
ALTER TABLE coude_players ADD COLUMN season INTEGER NOT NULL DEFAULT 1;
ALTER TABLE coude_players ADD COLUMN repos_last_used TIMESTAMPTZ;
```

### Nouvelle table `coude_dons`
```sql
CREATE TABLE coude_dons (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    donor_id TEXT NOT NULL,
    receiver_id TEXT NOT NULL,
    don_type TEXT NOT NULL,  -- 'coins' ou item_key
    quantity INTEGER NOT NULL,
    tax INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### Nouvelle table `coude_seasons`
```sql
CREATE TABLE coude_seasons (
    id SERIAL PRIMARY KEY,
    guild_id TEXT NOT NULL,
    season_number INTEGER NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ended_at TIMESTAMPTZ,
    champion_id TEXT,
    champion_name TEXT,
    UNIQUE(guild_id, season_number)
);
```

### Nouvelle table `coude_season_titles`
```sql
CREATE TABLE coude_season_titles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    guild_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    season_number INTEGER NOT NULL,
    title_key TEXT NOT NULL,  -- 'champion', 'best_fighter', 'thief_king', etc.
    title_label TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### Modification de `coude_combats`
```sql
-- Ajouter le suivi des rounds
ALTER TABLE coude_combats ADD COLUMN rounds_data JSONB;
ALTER TABLE coude_combats ADD COLUMN channel_id_temp TEXT;
ALTER TABLE coude_combats ADD COLUMN final_mise BIGINT;  -- mise apres surenchere
```

Le champ `rounds_data` stocke le detail de chaque round en JSON :
```json
[
    {
        "round": 1,
        "atk_roll": 14,
        "def_roll": 8,
        "atk_damage": 22,
        "def_damage": 7,
        "atk_hp_after": 143,
        "def_hp_after": 106,
        "chaos_event": null,
        "passif_triggered": "blindage"
    }
]
```

---

## Priorite d'implementation

### Phase 1 — Fondations
1. Nouvelle formule de degats (remplacement dans `combat.rs`)
2. HP reels + regeneration (modification `progression.rs` + `db.rs`)
3. Combat multi-rounds (refonte `combat.rs`)
4. Commande `/classe` + classes cachees
5. Fix des bugs existants (Rage, Casino XP, perte 100%)

### Phase 2 — Nouvelles mecaniques
6. Salons de combat temporaires
7. Systeme de surenchere
8. Vol interactif
9. Potions et items de soin au shop
10. Commande `/donner`

### Phase 3 — Casino et polish
11. Blackjack
12. Roue reworkee
13. Reset saisonnier
14. Messages fun et flavour texts partout
15. `/reset-stats` et `/repos`

---

> Ce document est la reference pour l'implementation de Coup de Coude v2.
> Chaque section peut etre implementee independamment en suivant l'ordre de priorite.
