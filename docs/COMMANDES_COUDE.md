# ⚔️ Coup de Coude — Guide des commandes

Bienvenue dans le **Coup de Coude**, le mini-jeu PvP du serveur !
Défie d'autres joueurs, pari sur les combats, vole, hérite de classes cachées et grimpe au leaderboard.

> 💡 Toutes les commandes commencent par `/` et s'utilisent dans les salons dédiés au jeu (demande à un admin si tu ne sais pas lesquels).

---

## 🚀 Démarrer

Dès que tu utilises une commande coude pour la première fois, ton profil est créé avec **200 coins** de départ et **100 PV**.

- **`/profil`** — affiche ton profil : niveau, classe, PV, ATK/DEF, coins, stats de combat.
- **`/profil user:@joueur`** — voir le profil de quelqu'un d'autre.

---

## ⚔️ Combattre un joueur

### `/coude cible:@joueur [mise:N] [special:…]`
Défie un joueur en combat.

- `cible` : le joueur à défier (obligatoire)
- `mise` : montant en coins à miser (défaut : config du serveur)
- `special` : objet spécial à utiliser (surprise, double_coup, coup_traitre, rage, poison, mindgame, bouclier, antidote)

> ✅ **Nouveau** : avant que le défi ne parte, le bot t'affiche d'abord tes **PV actuels** dans un message privé avec un bouton **Confirmer / Annuler**. Ça t'évite de te lancer dans un combat si tu es bas en PV sans t'en rendre compte. La mise et l'objet ne sont prélevés qu'après confirmation.

Le défenseur voit ensuite un message public avec **Accepter / Objet / Refuser / Annuler**.

### `/accepter`, `/refuser`, `/annuler`
Pas à taper à la main — ce sont les boutons qui apparaissent sur le défi.

- **Accepter** : le combat se lance, multi-rounds, classes, HP, chaos events.
- **Objet** : permet au défenseur de répondre avec un objet défensif (bouclier, antidote…).
- **Refuser** : annule proprement, aucune pénalité.
- **Annuler** (par l'attaquant) : pénalité configurée sur le serveur.

### `/hp`
Affiche tes PV actuels / max, ton palier de régen courant et l'estimation pour full heal.

### `/repos`
Regen **complète** tes PV. Cooldown **12h**. À utiliser avant un combat si tu es bas.

### ❤️ Régen passive des PV

Tes PV se régénèrent tout seuls au fil du temps, avec un **taux dégressif par palier** : plus tu es bas, plus tu récupères vite.

| Ton % HP | Taux | ≈ temps par palier (sur 100 HP) |
|---|---|---|
| 0 – 25 % | **+100 HP/h** | 15 min pour passer de 0 à 25 |
| 25 – 50 % | **+50 HP/h** | 30 min pour passer de 25 à 50 |
| 50 – 75 % | **+30 HP/h** | 50 min pour passer de 50 à 75 |
| 75 – 100 % | **+10 HP/h** | 2h30 pour finir de 75 à 100 |

👉 **Full heal depuis 0 HP : ~3h45 de régen passive**, dont les 50 premiers PV en ~45 min.
👉 `/repos` reste le raccourci full heal instantané (cooldown 12h).

> ⚠️ **Tu ne peux pas lancer `/coude` si tu es à moins de 10 % de tes PV.** Le bot refuse la commande et te suggère d'utiliser `/repos` ou d'attendre un peu.

---

## 💰 Économie

### `/donner user:@joueur montant:N [item:nom]`
Transférer des coins (ou un objet) à un autre joueur.

### `/voler cible:@joueur`
Tente un vol sur un autre joueur. C'est interactif : la victime peut se défendre avec un bouton si elle a les bons items. Cooldown + risque d'échec.

### `/shop`
Ouvre la boutique : bouclier, potions, attaques spéciales, antidotes, etc. Les achats sont débités immédiatement.

### `/prime cible:@joueur montant:N`
Mets une **prime** sur la tête d'un joueur. Le prochain combattant qui le bat récupère ta prime en plus de la mise normale.

### `/assurance`
Souscris une assurance combat : si tu perds ton prochain combat, tu récupères une partie de la mise. Valide pour X combats.

---

## 🎲 Parier sur les combats

### `/pari combat_id:<id> joueur:@joueur montant:N`
Quand un défi est lancé (phase "paris ouverts"), tu peux miser sur le joueur que tu penses gagnant.

- Si le joueur que tu as backé gagne → tu récupères **2× ta mise**.
- Si c'est une égalité → remboursement.
- Si perdu → mise perdue ; 10 % du pool de pertes est donné en bonus au vainqueur du combat.

> ⚠️ Les paris ferment au moment où le combat démarre — pas de pari après coup.

---

## 📈 Progression

### `/train stat:atk|def`
Dépense **1 point de stat** pour augmenter ton ATK ou ta DEF.
Tu gagnes des points de stat à chaque montée de niveau.

### `/reset-stats`
Redistribue **tous** tes points de stat (atk + def retournent dans ta réserve).
**Coût : 300 coins.**

### `/classe classe:<nom>`
Change ta classe de combat (bourrin, tacticien, chanceux, classes cachées à débloquer…).
Un sélecteur s'ouvre pour choisir.

### `/leaderboard`
Classement des top joueurs du serveur (coins + stats de combat).

---

## 🔍 Résumé / Debug

### ✨ `/resume [user:@joueur]` (nouveau !)

Affiche les **15 derniers mouvements de coins** d'un joueur (toi par défaut, ou la personne que tu mentionnes).

Pour chaque événement tu vois :

- la date + l'heure
- le type d'événement (combat gagné, pari placé, vol subi, achat shop, blackjack…)
- le montant (± coins)
- le solde **après** l'opération

Et en en-tête :

- ton solde **avant** le premier mouvement listé
- ton solde **après** ces mouvements
- les totaux **gains** et **pertes** sur la période
- ton solde actuel

> 🔧 Utile pour comprendre d'où viennent ou partent tes coins, et pour signaler un bug si un montant ne colle pas.

Tout le monde peut voir le résumé de tout le monde : transparence totale.

---

## 🗓️ Saison & events

### `/saison`
Infos sur la saison en cours : durée, récompenses, classement saisonnier.

### Chaos journalier
Un event aléatoire peut ponctionner des coins à un joueur au profit d'un autre, chaque jour. Pas de commande — ça arrive tout seul.

### Bloodbath
Event serveur ponctuel : pendant la durée de l'event, **tous les défis sont auto-acceptés**. Le défenseur n'a pas le choix !

---

## 🎯 Récap rapide

| Commande | Quoi |
|---|---|
| `/profil [user]` | Voir le profil d'un joueur |
| `/coude cible:@ ...` | Défier quelqu'un (avec confirmation PV) |
| `/hp` | Tes PV actuels |
| `/repos` | Regen PV complète (cd 12h) |
| `/shop` | Boutique |
| `/donner user: montant:` | Transfert de coins ou d'item |
| `/voler cible:` | Tentative de vol |
| `/prime cible: montant:` | Poser une prime |
| `/assurance` | Assurance combat |
| `/pari combat_id: joueur: montant:` | Parier |
| `/train stat:` | Dépenser un point de stat |
| `/classe classe:` | Changer de classe |
| `/reset-stats` | Redistribuer ses points (300 c) |
| `/leaderboard` | Classement |
| `/saison` | Infos saison |
| **`/resume [user]`** | **Historique des 15 derniers mouvements de coins** |

---

## ❓ Questions fréquentes

**J'ai perdu des coins sans comprendre, c'est un bug ?**
Lance `/resume` et regarde les 15 derniers mouvements. Tu verras exactement ce qui est entré/sorti. Si quelque chose ne colle toujours pas, ping un admin avec le screenshot.

**Je peux me défier moi-même ?**
Non.

**Je peux défier un bot ?**
Non.

**Pourquoi mon défi me demande de confirmer ?**
Pour t'éviter de partir en combat alors que tu es bas en PV sans t'en rendre compte. Si tu valides, la mise et l'objet éventuel sont prélevés ; si tu annules, rien n'est perdu.

**Qu'est-ce qui se passe si je ne réponds pas à un défi dans les 24h ?**
Tu es marqué "lâcheté" (compteur visible sur ton profil), tu perds 20 % de la mise, et les paris sont remboursés.

**Comment on gagne des XP ?**
Principalement en combat : +15 XP pour le gagnant (+30 si "giant killer"), +5 pour le perdant. Change de niveau = +3 points de stat + titre nouveau.

---

*Bon courage et que le meilleur coude gagne !* 🏆
