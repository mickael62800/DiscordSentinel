# Guide du jeu Coup de Coude

## C'est quoi ?

Coup de Coude est un jeu de combat, de strategie et de bluff integre a Discord. Defie d'autres joueurs, mise des coins, vole tes adversaires, joue au casino et grimpe dans le classement !

---

## Commandes principales

| Commande | Salon | Description |
|----------|-------|-------------|
| `/coude @joueur [mise] [special]` | Combats | Defier un joueur |
| `/pari @combattant mise` | Combats | Parier sur un combat en cours |
| `/profil [@joueur]` | Profil | Voir son profil ou celui d'un autre |
| `/leaderboard` | Leaderboard | Classements du serveur |
| `/voler @joueur` | Activites | Tenter de pickpocket quelqu'un |
| `/casino mise` | Activites | Jouer au casino |
| `/shop [acheter:item]` | Profil | Voir/acheter des objets |
| `/train atk/def` | Profil | Depenser des points de stats |
| `/assurance` | Profil | Acheter une assurance combat |
| `/prime @joueur montant` | Activites | Poser une prime sur quelqu'un |

---

## Les classes

Chaque joueur choisit une classe qui determine ses stats de depart et sa progression.

| Classe | ATK | DEF | Croissance ATK | Croissance DEF | Special |
|--------|-----|-----|----------------|----------------|---------|
| **Bourrin** | 25 | 8 | +4/niv | +1/niv | Frappe fort mais encaisse mal |
| **Agile** | 12 | 18 | +2/niv | +3/niv | 15% de chance d'esquiver |
| **Fourbe** | 18 | 14 | +3/niv | +2/niv | +20% de coins voles en combat |
| **Tank** | 8 | 25 | +1/niv | +4/niv | Lent mais increvable |

---

## Comment fonctionne un combat ?

### 1. Le defi
Tape `/coude @joueur mise:50` pour defier quelqu'un avec 50 coins en jeu.

Le defenseur voit 4 boutons :
- **Accepter** : Le combat est lance, les paris s'ouvrent
- **Objet** : Utiliser un objet defensif avant le combat
- **Refuser** : Compte comme lachete (+1 au compteur)
- **Annuler** : Seul l'attaquant peut annuler (penalite en coins)

### 2. Phase de paris
Apres acceptation, les autres joueurs ont **5 minutes** pour parier sur le vainqueur avec `/pari`.

### 3. Resolution
Le combat est resolu automatiquement par le serveur :

**Calcul des degats :**
```
Degats = max(5, (Roll x ATK_effectif / 50) - DEF_adverse)
```

- Chaque joueur lance un de 1-100
- Les stats (ATK/DEF) modifient le resultat
- Celui qui inflige le plus de degats gagne

**Marge de victoire :**
- Ecart < 10 degats : victoire serree → 60% de la mise
- Ecart 10-20 : victoire correcte → 80% de la mise
- Ecart > 20 : victoire nette → 100% de la mise

### 4. Resultats
- Le gagnant recoit les coins + XP
- Le perdant perd sa mise (reduite de 50% si assure)
- Les deux joueurs recoivent un DM avec le detail complet
- Les parieurs gagnants doublent leur mise

---

## Evenements Chaos (18% de chance)

Des evenements aleatoires peuvent bouleverser le combat !

| Evenement | Chance | Effet |
|-----------|--------|-------|
| **Critique Sauvage** | 5% | Le gagnant empoche **x3** les coins ! |
| **Esquive Divine** | 5% | Le defenseur contre-attaque automatiquement |
| **Accident Debile** | 3% | Les deux joueurs perdent leur mise |
| **Glissade** | 2% | L'attaquant se frappe lui-meme, le defenseur gagne |
| **Vol a la Tire** | 3% | Le gagnant recoit +20% de bonus |

---

## Le Shop

Achete des objets pour prendre l'avantage en combat !

| Objet | Prix | Effet |
|-------|------|-------|
| **Rage** | 100 | +50 points d'ATK pour ce combat |
| **Mindgame** | 150 | Voir le roll de l'adversaire |
| **Explosion** | 200 | Les deux joueurs perdent la mise (defenseur) |
| **Double Coup** | 250 | Lance le de 2 fois, garde le meilleur |
| **Attaque Surprise** | 300 | L'adversaire ne peut pas refuser le defi |
| **Coup Traitre** | 350 | Ignore completement la defense adverse |
| **Inversion** | 500 | Echange tes coins avec ceux de l'adversaire |

Les objets sont **consommables** : utilises une fois puis disparaissent.

---

## Le Vol

Commande : `/voler @joueur`

- **Taux de succes** : 30% de base (50% pour les Fourbes)
- **Gain** : 10 a 25% des coins de la cible (aleatoire)
- **Echec** : Tu perds 15% de tes propres coins
- **Cooldown** : 30 minutes entre chaque tentative
- La cible doit avoir au moins 10 coins

**XP** : +5 XP par vol reussi.

---

## Le Casino

Commande : `/casino mise:100`

| Resultat | Probabilite | Gain |
|----------|-------------|------|
| **Faillite totale** | 2% | Tu perds TOUS tes coins |
| **Perdu** | 50% | Tu perds ta mise |
| **x2** | 25% | Tu gagnes 2x ta mise |
| **x5** | 15% | Tu gagnes 5x ta mise |
| **JACKPOT x10** | 8% | Tu gagnes 10x ta mise ! (+10 XP) |

**Limites :**
- Cooldown entre chaque partie (defaut : 5 min)
- Nombre max de parties par jour (defaut : 10)
- Plafond de gains quotidiens (defaut : 5000 coins)

---

## L'Assurance

Commande : `/assurance`

- **Cout** : 50 coins
- **Duree** : 1 heure
- **Effet** : Reduit les pertes de combat de 50%
- **Risque** : 5% de chance que l'assurance soit une **arnaque** (dans ce cas, les pertes sont **doublees** !)

---

## Les Primes

Commande : `/prime @joueur montant:200`

Place une prime sur la tete d'un joueur. Le prochain qui le bat en combat recupere toute la prime ! Les primes se cumulent.

---

## Progression et Niveaux

### XP
- **Combat gagne** : +15 XP
- **Vol reussi** : +5 XP
- **Jackpot casino** : +10 XP

### Niveaux et Titres
| Niveau | Titre | XP cumule approximatif |
|--------|-------|----------------------|
| 1-4 | Debutant | 0 - 800 |
| 5-9 | Bagarreur | 800 - 4 500 |
| 10-14 | Guerrier | 4 500 - 12 000 |
| 15-19 | Veteran | 12 000 - 25 000 |
| 20-24 | Champion | 25 000 - 50 000 |
| 25 | Inarretable | 50 000+ |

A chaque niveau, tu gagnes **3 points de stats** a repartir en ATK ou DEF avec `/train`.

---

## Matchmaking

Le jeu empeche les combats trop desequilibres :

| Ecart de niveau | Effet |
|-----------------|-------|
| 0-2 niveaux | Pas de handicap |
| 3-5 niveaux | Le plus fort a -20% ATK |
| 6-9 niveaux | Le plus fort a -40% ATK |
| 10+ niveaux | Combat **bloque** |

**Bonus underdog** : Si le plus faible gagne malgre un ecart de 3+ niveaux, sa mise est **doublee** et son XP est **double** !

---

## Systeme de lachete

Si tu refuses trop de defis :
- A partir de **5 refus**, tu es marque comme **lache**
- Les laches gagnent **20% de moins** dans tous leurs combats
- Ton compteur de lachete est visible dans `/profil` et `/leaderboard`

---

## Classements

`/leaderboard` affiche 5 classements :

1. **Les plus riches** — par solde de coins
2. **Plus haut niveau** — par niveau et XP
3. **Plus gros voleurs** — par total de coins voles
4. **Les plus laches** — par nombre de refus
5. **Rois du chaos** — par nombre d'evenements chaos declenches

---

## Evenements serveur

### Happy Hour
Quand l'admin active le happy hour, **tous les gains de combat sont doubles** !

### Bloodbath
Pendant le bloodbath, **tous les defis sont auto-acceptes**. Impossible de refuser !

### Daily Chaos (La Roue du Destin)
Chaque jour, un joueur aleatoire **perd 20% de ses coins** qui sont transferes a un autre joueur aleatoire. C'est la vie.

---

## Conseils pour debuter

1. **Choisis ta classe** en fonction de ton style :
   - Aggressif → Bourrin
   - Defensif → Tank
   - Malin → Fourbe (bonus vol + coins)
   - Equilibre → Agile (chance d'esquive)

2. **Ne mise pas tout** — commence petit pour apprendre les mecaniques

3. **Utilise le shop** — un Double Coup ou une Rage peut faire la difference

4. **Assure-toi** avant les gros combats (mais attention aux arnaques !)

5. **Vole les riches** — 25% des coins d'un joueur riche = gros butin

6. **Parie malin** — observe les niveaux et classes avant de parier

7. **Casino avec moderation** — le x10 est tentant mais 52% de chance de perdre

---

*Coup de Coude | Sentinel — Bonne chance et que le meilleur gagne !*
