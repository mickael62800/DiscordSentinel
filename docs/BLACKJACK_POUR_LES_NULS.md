# ♠️ Blackjack — Guide pour les nuls

Tout ce qu'il faut savoir pour jouer au **Blackjack** sur le serveur. Mode solo (rapide) + tables multijoueur.

---

## 🚀 Démarrage en 30 secondes

### Mode solo (le plus rapide)
```
/blackjack mise:100
```
Le bot te tire 2 cartes, t'affiche les boutons **Hit / Stand / Double** → tu joues, le dealer joue, tu gagnes ou perds.

### Mode multijoueur (panel partagé)
1. Trouve le **panel Blackjack** posté par un admin (gros message avec "🃏 Jouer au Blackjack")
2. Clique sur **🃏 Jouer au Blackjack** → un **salon privé** est créé pour toi
3. Tu peux jouer seul ou inviter des amis dans ce salon

C'est tout. Les détails plus bas.

---

## 🎮 Les règles du blackjack (rappel)

Le but : **avoir une main qui se rapproche le plus de 21**, sans dépasser.

### Valeur des cartes
| Carte | Valeur |
|---|---|
| 2 → 10 | Leur valeur faciale (2, 3, …, 10) |
| Valet, Dame, Roi | **10** |
| **As** | **1 ou 11** (le bot choisit automatiquement le plus avantageux) |

### Le déroulement d'une partie

1. Tu mises X coins (entre `min_bet` et `max_bet`, défaut 10–1000)
2. Le dealer te donne **2 cartes face visible** + se donne **1 carte visible et 1 cachée**
3. Tu choisis :
   - **Hit** 🃏 → prendre une carte de plus
   - **Stand** ✋ → arrêter, le dealer joue
   - **Double** 💰 → doubler ta mise + 1 seule carte de plus + stop forcé (uniquement avec 2 cartes en main)
4. Tu peux Hit autant que tu veux. Si tu dépasses 21 → **bust** (tu perds)
5. Quand tu Stand, le dealer retourne sa carte cachée et tire jusqu'à atteindre **17 ou plus**
6. Comparaison des scores → résultat

### Les résultats possibles

| Status | Ce qui s'est passé | Payout |
|---|---|---|
| `player_blackjack` | Toi : 21 sur 2 cartes (As + 10/Valet/Dame/Roi) | **Mise × 2.5** (1.5x bonus + récup mise) |
| `player_win` | Toi > Dealer (sans bust) | **Mise × 2** (récup + gain égal à la mise) |
| `dealer_bust` | Dealer dépasse 21 | **Mise × 2** |
| `push` | Égalité (toi = Dealer) | **Mise rendue** (mise × 1) |
| `dealer_win` | Dealer > Toi (sans bust) | **0** (tu perds) |
| `player_bust` | Toi dépasses 21 | **0** (tu perds) |

### Le **Blackjack naturel** (le coup parfait)

Quand tes 2 premières cartes font 21 (As + figure), c'est un **blackjack naturel** = `player_blackjack` → payout **× 2.5** (mise rendue + 1.5× bonus).

Exemple : tu mises 100 coins, tu reçois As ♠️ + Roi ♥️ → tu gagnes **250 coins** (récup 100 + 150 de bonus).

> Le multiplicateur du blackjack naturel est configurable (`blackjack_payout`, défaut 1.5x).

---

## 🃏 Mode 1 — Solo (slash command)

C'est le mode le plus simple, rapide.

```
/blackjack mise:100
```

Le bot répond avec un embed **éphémère** (visible que par toi) qui montre :

```
Ta main : 7 ♣️ + 9 ♥️ = 16
Dealer  : 10 ♠️ + 🂠 (cachée)

[Hit] [Stand] [Double]
```

Tu cliques :
- **Hit** → tire une carte. Repeat jusqu'à Stand ou bust.
- **Stand** → le dealer joue, résultat affiché.
- **Double** → mise doublée + 1 carte + stop forcé. **Disponible uniquement sur les 2 premières cartes**.

L'embed se met à jour à chaque action. Le résultat final affiche les scores + payout.

### Une seule partie à la fois

Tu ne peux avoir **qu'une seule partie solo en cours**. Si tu re-tapes `/blackjack` alors que t'en as déjà une → erreur "Tu as déjà une partie en cours".

Pour reprendre ta partie en cours, vérifie ton historique ou tape simplement Hit/Stand sur l'embed précédent (les boutons restent actifs).

---

## 🎰 Mode 2 — Tables multijoueur (panel)

Pour jouer **avec d'autres joueurs sur la même table**, ou avoir un environnement persistant.

### Étape 1 : ouvre une table

L'admin a posé un panel quelque part avec `/blackjack-setup`. Click sur **🃏 Jouer au Blackjack** :
- Le bot crée un **salon Discord** rien que pour toi (`#bj-tonpseudo`)
- Personne d'autre ne le voit (sauf admins)

### Étape 2 : invite des amis (optionnel)

Dans ton salon, click sur **Inviter** puis tape le pseudo Discord ou l'@mention :
- L'invité reçoit accès au salon
- Tu peux jouer ensemble

Note : seul **toi** (le créateur de la table) peux inviter et fermer.

### Étape 3 : joue

Click sur **Jouer** → le bot te demande ta mise via un select :
```
Mise :  [10] [50] [100] [500] [1000]
```

Choisis → la partie démarre comme en solo, mais cette fois les autres joueurs peuvent voir le déroulement (sauf leurs cartes privées).

### Le shoe partagé

Sur les tables, il y a **6 jeux de cartes mélangés** (un "shoe" de 312 cartes). Toutes les parties consécutives consomment ce shoe. Quand il est presque vide (~1/3 restant), le bot le re-mélange automatiquement.

Avantage : règle casino réaliste, pas de comptage de cartes possible.

### Étape 4 : ferme la table

Quand t'as fini :
- Click sur **Fermer la table** dans ton salon
- Salon supprimé immédiatement

### Auto-fermeture AFK

Si la table reste **inactive 10 minutes** (pas de partie en cours, personne ne joue), le bot la ferme automatiquement. Configurable via `afk_timeout_secs` (défaut 600s).

---

## 💵 Les coins (porte-monnaie)

Le wallet est **partagé** entre tous les jeux du serveur :
- ♠️ Blackjack
- 🥊 Coup de Coude
- 🎰 Slot Machine
- 🎁 Récompenses admin

Tape `/profil` ou regarde sur la page web pour voir ton solde.

### Mises par défaut

| Paramètre | Défaut |
|---|---|
| `min_bet` | 10 coins |
| `max_bet` | 1000 coins |
| `starting_coins` | 200 coins (premier solde si t'en as jamais eu) |
| `blackjack_payout` | × 1.5 (sur 2 cartes 21) |

Les admins peuvent ajuster ces valeurs dans la config web.

### Si t'es à sec

Si tu cliques `/blackjack mise:500` alors que t'as 50 coins → erreur "Solde insuffisant".

Solutions :
- Joue petit (`mise:10`)
- Fais ton **Daily** sur Slot Machine (gratuit)
- Combats / vols sur Coude
- Demande à un admin de te recharger

---

## 🎯 Stratégie de base (pour gagner plus souvent)

Le blackjack est l'un des seuls jeux de casino où la stratégie compte vraiment. Voici la **stratégie de base simplifiée** (RTP ~99% si suivie) :

### Si tu as une **main dure** (sans As, ou As compté comme 1)

| Ta main | Carte visible du dealer | Action |
|---|---|---|
| ≤ 8 | Tout | **Hit** |
| 9 | 3-6 | **Double** |
| 9 | autre | Hit |
| 10-11 | < 10 | **Double** |
| 10-11 | 10 ou As | Hit |
| 12 | 4-6 | **Stand** |
| 12 | autre | Hit |
| 13-16 | 2-6 | **Stand** |
| 13-16 | 7+ | **Hit** |
| 17+ | Tout | **Stand** |

### Si tu as un **As** compté comme 11 (main souple)

| Ta main | Carte dealer | Action |
|---|---|---|
| As + 2-6 | Tout | Hit |
| As + 7 | 9, 10, As | Hit |
| As + 7 | autre | Stand |
| As + 8+ | Tout | Stand |

### Règles d'or

1. **Toujours Stand sur 17+** (sauf As+7 vs dealer fort)
2. **Toujours Hit sur ≤ 11** — impossible de buster
3. **Double sur 11** quand le dealer a une carte faible (2-9)
4. **Hit sur 12-16** si dealer ≥ 7 (il va probablement faire 17+, faut tenter)
5. **Stand sur 12-16** si dealer ≤ 6 (il a de bonnes chances de buster)

### Le double : utilisation maligne

Le **Double** doublera ta mise mais te limite à 1 seule carte. À utiliser quand :
- Tu as 11 (forte chance de tomber sur 21)
- Tu as 10 et le dealer est faible
- Tu as 9 et le dealer est très faible (3-6)

Ne double **jamais** quand t'as 13+ (trop de risque de buster avec 1 seule carte).

---

## ❓ FAQ

### "Je clique Hit mais rien ne se passe"
→ Le bot ou l'API est down. Réessaie dans quelques secondes ou ping un admin.

### "L'embed dit 'Partie expirée'"
→ Pas d'action depuis 30 min. La partie est annulée, ta mise t'est rendue automatiquement.

### "J'ai un As + Roi mais le payout est × 2 au lieu de × 2.5"
→ Le `blackjack_payout` est probablement réglé à 1.0 sur ce serveur (admin a viré le bonus). Demande à un admin de remettre 1.5.

### "Je peux split (séparer les cartes identiques) ?"
→ **Non**, le split n'est pas supporté pour l'instant. Hit / Stand / Double seulement.

### "Pourquoi le dealer s'arrête à 17 ?"
→ Règle classique du blackjack : le dealer **doit** tirer jusqu'à 17, puis s'arrête. Pas de choix pour lui (il est automatique).

### "Dealer a 17 souple (As + 6), il continue ou il s'arrête ?"
→ Sur cette implémentation, le dealer s'arrête à **17 dur OU souple**. Variante "stand on soft 17" (S17), considérée plus favorable au joueur.

### "Le shoe est presque vide ?"
→ Il se re-mélange automatiquement quand il reste ~1/3. Tu n'as rien à faire, transparent pour toi.

### "Mes amis voient mes cartes sur la table multijoueur ?"
→ Non — les cartes que tu reçois sont privées (ephemeral pour toi). Les autres voient juste les actions ("Alice a hit", "Alice score = 19").

### "Sur la table multijoueur, on joue en même temps ?"
→ Tour par tour. Quand c'est ton tour, t'as 30s pour agir, sinon Stand auto.

### "Combien de joueurs max sur une table ?"
→ Par défaut 7 (`DEFAULT_BLACKJACK_MAX_PLAYERS`).

---

## 🛠️ Pour les admins — config serveur

Va sur la page web → **Components** → **Blackjack**. Paramètres :

| Paramètre | Recommandé | Description |
|---|---|---|
| `enabled` | `true` | Active/désactive le module |
| `table_channel_id` | (un salon) | Salon dédié aux tables persistantes |
| `min_bet` | 10 | Mise minimale |
| `max_bet` | 1000 | Mise maximale |
| `afk_timeout_secs` | 600 | Auto-fermeture des tables inactives (en secondes) |

Pour `blackjack_payout`, `starting_coins`, `max_players_per_table` etc., ajuste les valeurs dans `bot_guild_config` directement si la page web ne les expose pas encore.

### Commandes admin

```
/blackjack-setup          # Pose le panel persistant dans le salon courant
```

---

## 🎲 Variants & limitations actuelles

### Implémenté ✅
- Mode solo (`/blackjack`)
- Mode multijoueur via tables (panel)
- Hit / Stand / Double
- Blackjack naturel (× 1.5 par défaut)
- Soft 17 (dealer s'arrête à 17 dur ou souple)
- Shoe partagé multi-decks (6 decks)
- Auto-fermeture AFK
- Wallet partagé avec autres jeux

### Pas implémenté ❌
- **Split** (séparer 2 cartes identiques en 2 mains)
- **Surrender** (abandonner et récupérer la moitié)
- **Insurance** (assurance contre blackjack dealer quand carte visible = As)
- **Tournois** programmés
- **Compteur de cartes** (le shoe se re-mélange automatiquement)

Tu veux qu'on en code une ? Demande à un dev / ouvre un ticket.

---

## 🐛 Bug ou suggestion ?

- Bug → ouvre un ticket avec `/ticket` ou ping un admin
- Suggestion → idem

---

**Bonne chance, et fais sauter la banque ! ♠️**
