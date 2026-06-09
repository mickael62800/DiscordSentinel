# 🃏 Blackjack — Guide du joueur

> Le casino du serveur ! Affronte le croupier, vise 21 sans dépasser, et fais fructifier tes **coins**.
> 💰 Tu joues avec **les mêmes coins que Coup de Coude** (un seul portefeuille pour tout le serveur).

---

## 🚀 Lancer une partie

1. Trouve le panneau **« Casino — Blackjack »** posté par un admin et clique sur **🃏 Jouer au Blackjack**.
2. Le bot te crée automatiquement un **salon privé** (`#bj-tonnom`), visible par toi seul.
3. Choisis ta **mise** (paliers proposés : 50 / 100 / 250 / 500 coins).
4. Les cartes sont distribuées, à toi de jouer !

> Tu peux aussi **inviter des amis** à ta table (jusqu'à 7 joueurs). Chacun joue sa propre main contre le même croupier.

---

## 🎯 Le but

Avoir une main dont le total est **le plus proche de 21**, sans le dépasser, et **battre le croupier**.

**Valeur des cartes :**
- Cartes **2 à 10** : leur valeur.
- **Valet, Dame, Roi** : 10.
- **As** : 11, ou 1 automatiquement si 11 te ferait dépasser 21.

---

## 🕹️ Tes actions

| Action | Quand | Effet |
|---|---|---|
| **🎯 Tirer** (Hit) | À tout moment | Prends une carte de plus |
| **✋ Rester** (Stand) | À tout moment | Tu t'arrêtes, le croupier joue |
| **💰 Doubler** (Double) | Seulement sur tes 2 premières cartes | Double ta mise, reçois 1 carte, puis fin automatique |

*(Pas de split ni d'assurance dans cette version.)*

---

## 🤖 Le croupier

Une fois que tu restes, le croupier retourne sa carte cachée et **tire tant qu'il a moins de 17**. Il s'arrête à 17 ou plus. Ses règles sont fixes : il n'a aucun choix.

---

## 💵 Gains et pertes

| Situation | Ce que tu touches |
|---|---|
| **Blackjack naturel** (21 avec tes 2 premières cartes) | **×2,5** ta mise 🎉 |
| **Tu gagnes** (meilleur score que le croupier, ou croupier qui dépasse 21) | **×2** ta mise |
| **Égalité** (même score) | Ta mise t'est **rendue** |
| **Tu dépasses 21** (bust) ou score inférieur | Tu **perds** ta mise |

> Exemple : tu mises 100 et tu fais blackjack → tu repars avec 250 coins.

---

## ⏱️ Bon à savoir

- **Pas de cooldown ni de limite** : tu peux enchaîner les parties tant que tu as des coins.
- Mise **minimum 10**, **maximum 1000** coins (réglages par défaut).
- Ta table se **ferme toute seule après 30 minutes d'inactivité** (un message te prévient avant la suppression du salon).
- Attention à la **faillite** : si ton solde tombe à 0, le serveur ne se privera pas de te charrier 😏.

---

**Bonne chance au tapis !** 🃏
