# 🎰 Machine à sous (Slot) — Guide pour les nuls

Un guide simple et complet pour jouer à la **machine à sous** sur le serveur Discord. Aucune connaissance préalable requise.

---

## 🚀 Démarrage en 30 secondes

1. Trouve le **panel** posté par un admin dans un salon (gros message avec un bouton "🎰 Ouvrir ma machine")
2. Clique sur **🎰 Ouvrir ma machine**
3. Le bot crée pour toi un **salon privé** (`#slot-tonpseudo`) que tu es le seul à voir
4. Va dans ce salon, clique sur **🎰 Tirer** → la machine tourne 6 secondes, tu vois le résultat
5. Clique encore et encore. Quand t'as fini : **❌ Fermer**

C'est tout. Le reste de ce guide explique les détails.

---

## 🎮 Comment ça marche

### Le panel global (un par serveur)

L'admin pose un panel à un endroit avec `/slot-setup`. Ce panel est **public** et permanent. Il a un seul bouton :

```
🎰 Ouvrir ma machine
```

### Ton salon privé

Quand tu cliques, le bot **crée un salon Discord** rien que pour toi. Personne d'autre ne le voit (sauf les admins du serveur).

Dans ce salon tu trouves un message d'accueil + 3 boutons :

| Bouton | Effet |
|---|---|
| 🎰 **Tirer** | Lance un spin payant à la mise par défaut (50 coins par défaut) |
| 🎁 **Daily Bonus** | Spin **gratuit**, 1 fois par jour |
| ❌ **Fermer** | Supprime ton salon (tu peux toujours en rouvrir un après) |

### L'animation suspense

Click sur Tirer → tu vois ça pendant **6 secondes** :

```
t=0s  → 🎰  🎰  🎰     "La machine tourne..."
t=2s  → 🍒  🎰  🎰     1er symbole révélé
t=4s  → 🍒  🍋  🎰     2e symbole révélé
t=6s  → 🍒  🍋  🍊     3e symbole + résultat final
```

Le bot poste ensuite **un nouveau message en bas** avec les 3 boutons → t'es prêt pour le prochain spin sans devoir scroller.

---

## 💰 Les gains

Tu mises X coins. À la fin du spin, selon les 3 symboles tirés :

### Cas n°1 — **3 symboles identiques** (gain × multiplicateur)

| Symboles | Multiplicateur (par défaut) |
|---|---|
| 🍒 🍒 🍒 | **× 2** |
| 🍋 🍋 🍋 | **× 3** |
| 🍊 🍊 🍊 | **× 5** |
| 🍇 🍇 🍇 | **× 8** |
| 🔔 🔔 🔔 | **× 12** |
| ⭐ ⭐ ⭐ | **× 25** |
| 7️⃣ 7️⃣ 7️⃣ | **× 100 + JACKPOT** 💎 |

Exemple : tu mises 100 coins, tu tires 🔔🔔🔔 → tu gagnes **1200 coins** (100 × 12).

### Cas n°2 — **2 symboles identiques** (mise rendue)

Tu récupères ta mise, comme si t'avais pas joué. Pas de gain mais pas de perte.

Exemple : 🍒 🍒 🍋 → tu récupères tes 100 coins.

### Cas n°3 — **3 symboles différents** (perdu)

Tu perds ta mise.

Exemple : 🍒 🍋 🍊 → tu perds 100 coins.

---

## 💎 Le Jackpot progressif

C'est **le gros lot**.

### Comment alimenter le pot

À chaque spin (peu importe gagné ou perdu), **1% de ta mise** part dans un **pool jackpot commun à tout le serveur**. Le pool grossit en continu.

### Comment décrocher le jackpot

Tu dois tirer **3 × 7️⃣** (le symbole le plus rare, ~1% de chance par spin, donc ~0.0001% pour 3).

Quand tu décroches le jackpot tu remportes :
- **Mise × 100** (multiplicateur du symbole 7️⃣)
- **+ Tout le pool jackpot accumulé**

Exemple : pool à 25 000 coins, tu mises 100 et tu tires 3×7 → tu gagnes **100 × 100 + 25 000 = 35 000 coins**.

Après le jackpot, le pool **reset** à sa valeur de départ (1000 coins par défaut).

---

## 🎁 Le Daily Bonus

Chaque jour, tu peux faire **1 spin gratuit**. La mise est de 100 coins (configurable) mais **tu ne paies rien** — c'est le serveur qui paie.

Tu gardes les gains comme un spin normal. Si tu décroches le jackpot avec ton spin gratuit → t'es chanceux 🍀

Le compteur reset à minuit (heure du serveur).

---

## 💵 Les coins (porte-monnaie)

### D'où viennent tes coins ?

Tu partages ton **wallet** avec les autres jeux du serveur :
- 🥊 **Coup de Coude** (combats, paris, vols, braquages)
- ♠️ **Blackjack**
- 🎁 **Daily** des autres jeux
- 🎯 **Récompenses** que les admins t'attribuent

Tape `/profil` ou regarde sur la page web du serveur pour voir ton solde.

### Combien tu peux miser ?

Par défaut sur chaque serveur :
- **Mise minimum** : 10 coins
- **Mise maximum** : 1000 coins
- **Mise par défaut** (clic Tirer) : 50 coins

Les admins peuvent ajuster ces seuils.

### Si t'es à sec

Tu peux pas miser plus que ton solde. Si tu cliques Tirer alors que t'as 5 coins → erreur "Solde insuffisant".

Solutions :
- Faire ton **Daily Bonus** quotidien (gratuit)
- Jouer aux autres jeux (coude / blackjack)
- Demander un don à un autre joueur (`/donner` côté coude)
- Attendre que les admins te rechargent

---

## ⏱️ Cooldown

Entre deux spins, il y a **5 secondes** de cooldown (par défaut). Si tu cliques trop vite tu auras un message "Cooldown actif : encore X secondes".

C'est pour éviter le spam. Les admins peuvent l'ajuster (ou désactiver).

---

## 🚪 Fermer ton salon

Quand t'as fini de jouer :
- Click sur **❌ Fermer** sous n'importe quel résultat
- Le salon est supprimé immédiatement

Tu peux toujours en rouvrir un en cliquant sur le panel global. Ton solde de coins est sauvegardé dans le wallet partagé, donc pas de perte de progression.

> **Attention** : seul **toi** (ou un admin) peut fermer ton salon. Si quelqu'un d'autre clique → erreur de permission.

### Auto-close (V2 à venir)

Plus tard, le bot fermera automatiquement les salons inactifs au bout de X heures. Pour l'instant, faut le faire à la main.

---

## ❓ FAQ

### "J'ai cliqué Tirer mais rien ne se passe"
→ Regarde les logs côté admin. Probable : le bot est down, l'API est down, ou ton solde est à 0.

### "Le résultat n'apparaît pas après 6 secondes"
→ Discord a peut-être perdu la connexion à mi-animation. Re-clique Tirer, le précédent spin est déjà compté côté serveur (vérifie ton solde).

### "Le pool jackpot affiche toujours la même valeur"
→ Faut d'autres joueurs qui spinnent pour qu'il monte. 1% par mise × tous les joueurs du serveur. Si t'es seul à jouer ça monte lentement.

### "J'ai tiré 3×7 mais je vois pas le 'JACKPOT'"
→ Vérifie le titre de l'embed final : il doit dire "🎰 JACKPOT 🎰" en or. Si c'est juste "Gagné !" c'est que le 7️⃣ n'est pas configuré comme le **dernier** symbole de la liste (cf. config admin).

### "Je peux miser plus que 1000 ?"
→ Demande à un admin d'augmenter `max_bet` dans la config web (page Components → Slot Machine).

### "Je peux choisir ma mise ?"
→ Pas pour l'instant — le bouton Tirer utilise toujours la mise par défaut configurée par l'admin. La V2 ajoutera des boutons "Tirer ×2", "×5", "×10" ou un input mise libre.

### "Mes amis peuvent voir mes spins ?"
→ Non. Ton salon est privé. Seul **toi** + les **admins du serveur** (ADMINISTRATOR Discord) le voient.

### "Si je perds ma connexion pendant l'animation ?"
→ Le spin est calculé côté serveur **avant** l'animation. Donc même si l'animation foire, tes coins sont déjà débités/crédités correctement. Recharge ton solde pour vérifier.

---

## 🛠️ Pour les admins — config serveur

Va sur la page web du serveur → **Components** → **Slot Machine**. Tu peux régler :

| Paramètre | Recommandé | Description |
|---|---|---|
| `enabled` | `true` | Active/désactive le module |
| `min_bet` / `max_bet` | 10 / 1000 | Bornes de la mise |
| `default_bet` | 50 | Mise du bouton Tirer |
| `cooldown_secs` | 5 | Anti-spam entre 2 spins |
| `symbols` | `🍒,🍋,🍊,🍇,🔔,⭐,7️⃣` | CSV. Le **dernier** = jackpot |
| `weights` | `30,25,20,15,7,2,1` | Probabilités relatives |
| `payout_3x_multipliers` | `2,3,5,8,12,25,100` | Multiplicateurs 3 identiques |
| `payout_2x_enabled` | `true` | 2 identiques = mise rendue |
| `jackpot_pool_share_pct` | `1` | % de chaque mise → pool |
| `jackpot_starting_pool` | `1000` | Reset après jackpot décroché |
| `daily_bonus_enabled` | `true` | Daily 1 spin gratuit |
| `daily_bonus_mise` | `100` | Mise du daily |

---

## 🎯 Stratégie

C'est un jeu de **hasard** — la maison gagne toujours statistiquement. Mais voici quelques tips :

1. **Toujours faire le daily** : c'est gratuit, autant le tenter
2. **Petites mises répétées** > grosses mises rares (variance plus faible)
3. **Le jackpot est rare** (~1 sur 1 million) : ne mise pas tout dessus
4. **Surveille le pool** : plus il est gros, plus le jackpot vaut le coup
5. **Le 2-of-a-kind** te ramène ta mise → ce n'est pas une perte, juste pas un gain

**Espérance mathématique typique** (avec les défauts) : ~95% de retour. Tu perds ~5% de chaque mise sur le long terme. Joue pour le fun, pas pour faire fortune.

---

## 🐛 Bug ou suggestion ?

- Bug → ouvre un ticket avec `/ticket` ou ping un admin
- Suggestion → idem, ou propose-la à un admin qui pourra ajuster la config

---

**Bonne chance ! 🍀🎰**
