# ✊ COUP DE COUDE — Regles du Jeu

*Un mini-jeu social volontairement injuste, imprevisible et addictif.*

---

## 🎯 C'est quoi ?

Coup de Coude est un jeu de combat social ou tu mises des coins contre d'autres joueurs. Le but : accumuler des coins, voler ceux des autres, et survivre au chaos.

**Tu commences avec 200 coins.** Bonne chance, t'en auras besoin.

---

## ⚔️ COMMANDES

### Combat
| Commande | Description |
|----------|-------------|
| `/coude @joueur [mise]` | Lancer un defi (mise par defaut : 10) |
| `/profil [joueur]` | Voir ton profil ou celui d'un autre |
| `/leaderboard` | Classement chaotique |

### Argent
| Commande | Description |
|----------|-------------|
| `/casino <mise>` | Tente ta chance a la roulette |
| `/voler @joueur` | Tente de pickpocket quelqu'un |
| `/pari @combattant <mise>` | Parie sur le resultat d'un combat |
| `/assurance` | Achete une protection (50 coins, 1h) |

### Boutique & Primes
| Commande | Description |
|----------|-------------|
| `/shop` | Voir les objets disponibles |
| `/shop acheter <objet>` | Acheter un objet |
| `/prime @joueur <montant>` | Mettre une prime sur quelqu'un |

---

## 🥊 COMMENT CA MARCHE ?

### Lancer un combat
```
/coude @Darkponey 50
```
Tu defies un joueur avec une mise. Il recoit un message avec deux boutons :
- **✊ Accepter** — Le combat commence
- **🐔 Refuser** — Il perd 20% de la mise + gagne en lachete

⏰ **Le defenseur a 24h pour repondre.** Sinon c'est considere comme un refus.

### Resolution du combat
Les deux joueurs lancent un de (1-100) + bonus de classe.
Le plus haut gagne et empoche la mise du perdant.

**Mais attention...** a chaque combat, il y a **18% de chance** qu'un evenement chaotique se declenche.

---

## 🎲 SYSTEME CHAOS

A chaque combat, la roue du chaos peut tourner :

| Evenement | Chance | Effet |
|-----------|--------|-------|
| 💥 **Critique Sauvage** | 5% | Le gagnant empoche **x3** la mise |
| ✨ **Esquive Divine** | 5% | Le defenseur contre-attaque automatiquement |
| 💩 **Accident Debile** | 3% | Les **deux** perdent toute la mise |
| 🩴 **Glissade** | 2% | L'attaquant se frappe lui-meme |
| 💰 **Vol a la Tire** | 3% | Le gagnant vole **+20%** bonus |

*18% de chance que le chaos frappe. Personne n'est safe.*

---

## 🧬 CLASSES

Chaque joueur a une classe qui influence ses combats. **Elles ne sont PAS equilibrees.** C'est le but.

| Classe | Emoji | Attaque | Defense | Special | Description |
|--------|-------|---------|---------|---------|-------------|
| **Bourrin** | 💪 | +30 | -15 | — | Frappe fort mais encaisse mal |
| **Agile** | 🏃 | -10 | +25 | 15% esquive | Esquive souvent mais frappe faible |
| **Fourbe** | 🗡️ | +5 | +5 | +20% vol | Manipule les regles |
| **Tank** | 🛡️ | -20 | +35 | — | Lent mais increvable |

---

## 🛒 BOUTIQUE

Des objets pour renverser la situation (ou la rendre pire).

| Objet | Prix | Effet |
|-------|------|-------|
| 💣 **Explosion** | 200 | Les deux joueurs perdent toute la mise |
| 🔄 **Inversion** | 500 | Echange tes coins avec ceux de l'adversaire |
| 🧠 **Mindgame** | 150 | Vois le roll de l'adversaire avant de jouer |
| 😡 **Rage** | 100 | +50 attaque mais -50 defense |
| 💨 **Attaque Surprise** | 300 | L'adversaire ne peut PAS refuser |
| ✊✊ **Double Coup** | 250 | Lance le de deux fois, garde le meilleur |
| 🗡️ **Coup Traitre** | 350 | Ignore le bonus de defense adverse |

*Utilise un objet avec* `/coude @joueur [mise] special:<objet>`

---

## 🎰 CASINO

```
/casino 50
```

Tente ta chance. Resultats possibles :

| Resultat | Chance | Effet |
|----------|--------|-------|
| 💀 Perdu | 50% | Tu perds ta mise |
| 💰 x2 | 25% | Tu doubles ta mise |
| 🔥 x5 | 15% | Tu quintuples ta mise |
| 🎰 **JACKPOT x10** | 8% | Dix fois ta mise ! |
| ☠️ **FAILLITE TOTALE** | 2% | Tu perds **TOUS** tes coins |

*Le casino ne connait pas la pitie.*

---

## 🎲 PARIS

```
/pari @joueur 50
```

Parie sur le resultat du combat d'un autre joueur.
- Si ton poulain gagne → tu recuperes ta mise + ta part des paris perdants
- Si il perd → tu perds ta mise
- Si chaos "Accident Debile" → tout le monde perd

*Cree du drama entre spectateurs.*

---

## 🗡️ VOL

```
/voler @joueur
```

Tente de pickpocket un joueur.
- **30% de reussite** → tu voles 10-25% de ses coins
- **70% d'echec** → tu perds 15% de TES coins + message humiliant
- Cooldown : **30 minutes**
- La classe **Fourbe** a **50%** de chance de reussite

*Haut risque, haute recompense.*

---

## 🛡️ ASSURANCE

```
/assurance
```

Coute **50 coins**, dure **1 heure**.
- Si tu perds un combat → tu ne perds que **50%** de la mise
- **MAIS** il y a **5% de chance** que l'assurance soit une **ARNAQUE**
- Si c'est une arnaque → tu perds le **DOUBLE** de la mise

*Fais-tu confiance a l'assurance ?*

---

## 🎯 PRIMES

```
/prime @joueur 100
```

Met une recompense sur la tete de quelqu'un. Quiconque le bat en combat empoche la prime en plus de la mise.

*Les primes se cumulent. La chasse est ouverte.* 🔥

---

## 🐔 SYSTEME DE LACHETE

Tu refuses trop de combats ? Tu deviens un **lache officiel**.

- Chaque refus → +1 lachete
- **5+ refus** → role 🐔 Lache
- Les laches gagnent **20% de moins** en combat
- Message public humiliant a chaque refus

*Ose ou assume.*

---

## 🌪️ DAILY CHAOS

Chaque jour, a une heure aleatoire, **la Roue du Destin tourne** :
- Un joueur random **perd 20%** de ses coins
- Un autre joueur random **gagne** ce montant

*Personne n'est safe. Meme en dormant.*

---

## ⚡ EVENEMENTS SERVEUR

De temps en temps, des evenements speciaux se declenchent :

| Evenement | Effet |
|-----------|-------|
| ⚡ **Happy Hour** | Tous les gains **x2** |
| 🩸 **Bloodbath** | Tous les combats sont auto-acceptes (pas de refus) |
| 🎁 **Drop** | Distribution aleatoire de coins |

---

## 🏆 LEADERBOARD

Pas seulement les riches. On celebre aussi :
- 👑 **Les plus riches** — Top coins
- 🗡️ **Les plus gros voleurs** — Top coins voles
- 🐔 **Les plus laches** — Top refus
- 🌪️ **Les rois du chaos** — Top evenements chaos subis

---

## ⚠️ PHILOSOPHIE

> **Injuste = Fun.** Imprevisible = Addictif. Social = Vivant.

Si tu rales mais que tu continues a jouer → on a gagne. 😈

---

*Coup de Coude — Sentinel Discord*
