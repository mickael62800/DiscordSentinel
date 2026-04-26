# 📜 Coup de Coude — Toutes les commandes

> **42 commandes** au total, regroupées en 7 catégories. Toutes sont des slash commands (`/nom`) sauf indication contraire (les boutons sont des composants interactifs liés à `/coude`).

---

## 🗂️ Vue d'ensemble par catégorie

| Catégorie | Nombre | Commandes |
|---|---|---|
| ⚔️ Combat | 6 | `/coude`, `/coude-amical` + boutons `accepter`/`refuser`/`annuler`/`defend-item` |
| 💰 Économie | 6 | `/donner`, `/shop`, `/cagnotte`, `/memorial`, `/tout-ou-rien`, `/braquage` |
| 🥷 Vol & Protection | 4 | `/voler`, `/protection`, `/boost-voleur`, `/assurance` |
| 📈 Progression | 13 | `/profil`, `/hp`, `/train`, `/repos`, `/potion`, `/reset-stats`, `/classe`, `/leaderboard`, `/resume`, `/aide`, `/prestige`, `/ultimate`, `/saison` |
| 🎭 Social toxique | 9 | `/pari`, `/maudire`, `/prank`, `/saboter`, `/vendetta`, `/honneur`, `/coalition`, `/contribuer-prime`, `/prime` |
| 🔒 Prison | 1 | `/travaux` |
| ⚙️ Configuration | 3 | `/no-taunts`, `/taunts-channel`, `/progression-resync` |

---

## ⚔️ Combat

### `/coude`
- **Description** : Lance un défi contre un joueur avec mise en coins.
- **Options** : `cible` (User, requis), `mise` (Integer ≥1, **optionnel** — si omis, propose 20 % / 50c / 100c / all-in)
- **Effet** : Crée un combat avec mise en jeu, phase de défense via bouton, animation pour les grosses mises.
- **Accès** : tous

### `/coude-amical`
- **Description** : Duel d'entraînement sans mise — pour tester sans risque.
- **Options** : `cible` (User, requis)
- **Effet** : Combat zéro-risque (aucun coin transféré), gains XP séparés (+20 winner, +5 loser), stats de duels amicaux distinctes.
- **Accès** : tous

### Boutons de combat (composants `/coude`)
- **`Accepter`** — bascule le combat en phase de paris, démarre le compte à rebours.
- **`Refuser`** — perte de % mise + lâcheté +1, paris remboursés.
- **`Annuler`** — l'attaquant abandonne, perte de % mise.
- **`Défense par objet`** — select menu pour choisir un item défensif (rage, explosion, antidote, bouclier…). Consomme l'item, applique l'effet, résout immédiatement (bypasse la phase paris).

---

## 💰 Économie

### `/donner`
- **Description** : Donne des coins ou des items à un autre joueur.
- **Options** : `cible` (User), `type` (`coins` ou `items`), `quantite` (défaut 1)
- **Effet** : Transfert avec **10 % de taxe** (alimente la cagnotte). Items : un par un avec rollback si échec partiel.
- **Accès** : tous

### `/shop`
- **Description** : Boutique — attaque, défense ou braquage.
- **Options** : sub-commands `attaque` / `defense` / `braquage` + `acheter` (optionnel)
- **Effet** : Affiche le catalogue ou achète directement. Réduit le solde, ajoute à l'inventaire, alimente la cagnotte.
- **Accès** : tous

### `/cagnotte`
- **Description** : Affiche l'argent accumulé dans la caisse communautaire.
- **Effet** : Lecture seule — solde, total collecté, total redistribué, date de dernière redistribution.
- **Accès** : tous

### `/memorial`
- **Description** : Mémorial des clodos — top 10 plus grosses pertes au tout-ou-rien.
- **Effet** : Classement public des plus grosses ruines (-80 % au `/tout-ou-rien`).
- **Accès** : tous

### `/tout-ou-rien`
- **Description** : Mise tout ton wallet sur un 50/50 (1× par semaine, irréversible).
- **Effet** : Animation 10s puis verdict. **Victoire** : ×2 du wallet + annonce serveur. **Défaite** : -80 % + entrée Mémorial. Cooldown 1 semaine.
- **Accès** : tous

### `/braquage`
- **Description** : Tente de braquer la caisse communautaire (1× par semaine, gros risque).
- **Effet** : 5 % chance de base + 5 % par item consommé (cap 50 %). **Réussite** : empoche 30-75 % de la cagnotte. **Échec** : prison 24h + items perdus. Cooldown 1 semaine.
- **Accès** : tous

---

## 🥷 Vol & Protection

### `/voler`
- **Description** : Tente un pickpocket sur un joueur.
- **Options** : `cible` (User, requis)
- **Effet** : 60s pour la victime de cliquer « Se défendre » (malus AFK si timeout). Roll d20 + bonus (classe + boosts secrets). 15-25 % des coins volés si succès. Victime gagne +3 XP si elle bloque.
- **Accès** : tous

### `/protection`
- **Description** : Souscris un abonnement anti-vol (secret, éphémère).
- **Options** : `item` (chien_garde → forteresse), `duree` (1d / 3d / 5d / 7d, dégressif)
- **Effet** : Abonnement invisible côté voleur. 25-70 % chance de bloquer un vol selon l'item. Coût ≈ 50-500c (alimente la cagnotte).
- **Accès** : tous

### `/boost-voleur`
- **Description** : Souscris un abonnement boost voleur (secret, éphémère).
- **Options** : `item` (crochet → équipe_de_pros), `duree` (1d / 3d / 5d / 7d)
- **Effet** : +5 à +25 au roll de vol (cumulatif). Invisible côté victime. Coût ≈ 100-1500c.
- **Accès** : tous

### `/assurance`
- **Description** : Souscris une assurance temporaire contre les pertes de combat.
- **Options** : `duree` (1 jour | 1 semaine 6× | 1 mois 22×)
- **Effet** : Réduit les pertes de combat de 50 %. Consommable. **Niveau 5+** : jusqu'à 2 assurances simultanées. ~5 % de chance d'être un scam (cosmétique).
- **Accès** : tous

---

## 📈 Progression

### `/profil`
- **Description** : Affiche ton profil Coup de Coude.
- **Options** : `user` (optionnel, défaut = soi)
- **Effet** : Méga-fiche : HP / ATK / DEF / XP, solde, W/L/D, classe + passif, paliers, inventaire, assurance active, malédictions / sabotages, succès cosmétiques, saison.
- **Accès** : tous

### `/hp`
- **Description** : Affiche tes points de vie actuels.
- **Effet** : Barre HP visuelle (20 segments), couleur selon %, regen rate du palier actuel, estimation full-heal. Seuil KO < 10 %.
- **Accès** : tous

### `/train`
- **Description** : Dépense un point de stat pour améliorer ATK ou DEF.
- **Options** : `stat` (`attaque` | `defense`)
- **Effet** : Consomme 1 point → +1 ATK ou +1 DEF (recalcule HP effectif). Points gagnés via XP / level-up.
- **Accès** : tous

### `/repos`
- **Description** : Repose-toi pour récupérer tous tes HP (cooldown 12h).
- **Effet** : Full heal instantané. Cooldown 12h (8h si palier Convalescence niv. 15+).
- **Accès** : tous

### `/potion`
- **Description** : Utilise une potion de soin hors combat.
- **Options** : `type` (`potion_soin` +30 HP | `potion_majeure` +80 HP)
- **Effet** : Consomme la potion, +30 ou +80 HP (clamp max). Refuse si la potion serait gaspillée.
- **Accès** : tous

### `/reset-stats`
- **Description** : Redistribue tous tes points de stats (300 coins).
- **Effet** : Remet ATK/DEF à 0, restitue tous les points dépensés. Coût 300c (alimente la cagnotte). Atomique.
- **Accès** : tous

### `/classe`
- **Description** : Choisis ou change ta classe de combat.
- **Options** : boutons `bourrin` / `agile` / `fourbe` / `tank` après confirmation
- **Effet** : 1ᵉʳ choix gratuit, changements suivants payants. Cooldown 7 jours. Stats de base + passif distincts par classe.
- **Accès** : tous

### `/leaderboard`
- **Description** : Classement Coup de Coude.
- **Effet** : Top 5 sur 5 axes — richesse, niveau, voleurs, lâches, rois du chaos.
- **Accès** : tous

### `/resume`
- **Description** : Résumé des derniers mouvements de coins d'un joueur.
- **Options** : `user` (optionnel)
- **Effet** : Historique des 15 dernières transactions (combat gagné/perdu, pari, vol, etc.) avec solde avant/après.
- **Accès** : tous

### `/aide`
- **Description** : Suggestions contextuelles selon l'état de ton compte.
- **Effet** : 3-6 tips dynamiques (HP bas, points libres, pas de classe, premier combat, solde maigre, roue, cagnotte, profil, saison). Pure UX.
- **Accès** : tous

### `/prestige`
- **Description** : Active un prestige (niveau 25+, reset au niveau 1, +5 % gains permanents).
- **Effet** : Reset niveau à 1, +5 % gains permanent (cumul, max 5 prestiges = ⭐⭐⭐⭐⭐). Annonce serveur.
- **Accès** : tous (gating sur niveau 25+)

### `/ultimate`
- **Description** : Affiche ou active ton ultimate (débloqué au niveau 10).
- **Options** : `activer` (Boolean, défaut false)
- **Effet** : Par classe — **Bourrin** : swap HP avec l'adversaire. **Agile** : pile ou face instantané. **Fourbe** : vole la mise et fuit. **Tank** : statue (ne fait ni ne prend de dégâts, gagne par forfait). Cooldown 7 jours.
- **Accès** : tous (gating sur niveau 10+)

### `/saison`
- **Description** : Affiche les infos de la saison en cours.
- **Effet** : Durée 90 jours, top 3 sur 3 axes (richesse, niveau, voleurs). Auto-reset à la fin, champion proclamé.
- **Accès** : tous

---

## 🎭 Social toxique

### `/pari`
- **Description** : Parie sur l'issue du combat d'un joueur.
- **Options** : `combattant` (User), `mise` (Integer ≥1)
- **Effet** : Pari pendant la phase betting. Gagnants → mise × bonus. Perdants → remboursés. Plusieurs paris cumulables. Déclenche taunts en cas de faillite.
- **Accès** : tous

### `/maudire`
- **Description** : Pose une malédiction ridicule sur un pote pendant 24h (300c).
- **Options** : `cible` (User), `type` (optionnel — chicken / banana / leaky_wallet / slowness / insomnia / heartbreak ; aléatoire sinon)
- **Effet** : 1 malédiction active max par cible. 24h de durée. Levable contre 600c. Effets : pseudo poulet, 30 % rate les d20, 10c de frais par transaction, lenteur des messages, +50 % taunts défaite, licorne bloquée.
- **Accès** : tous

### `/prank`
- **Description** : Outils de troll communautaires.
- **Options** : `type` (`braquage` / `scoop` / `appel`), `cible` (optionnel pour braquage, requis sinon)
- **Effet** : **Braquage** (100c) → fausse alerte cagnotte. **Scoop** (200c) → fake news sur la cible. **Appel** (50c) → faux DM système promettant des coins fictifs. Zéro gameplay, pure ambiance.
- **Accès** : tous

### `/saboter`
- **Description** : Sabotages ciblés contre un autre joueur.
- **Options** : `type` (`pancarte` / `graisser` / `empoisonner` / `fausse_assurance`), `cible`
- **Effet** : **Pancarte** (150c, 7j) — tag « Rival officiel ». **Graisser** (200c) — sa prochaine attaque spéciale rate. **Empoisonner** (400c) — 10 % de ses 3 prochains gains te reviennent. **Fausse assurance** (500c) — si la cible perd avec assurance, elle est annulée + 200c pour toi.
- **Accès** : tous

### `/vendetta`
- **Description** : Déclare une vendetta officielle contre un joueur (7 jours).
- **Options** : `cible` (User)
- **Effet** : Si tu bats la cible dans la fenêtre, +100 % de gain bonus. Sinon humiliation publique (ton pseudo prend le suffixe « le Bourreau de @toi » côté cible).
- **Accès** : tous

### `/honneur`
- **Description** : Invoque la dette d'honneur contre un joueur qui te refuse trop.
- **Options** : `cible` (User)
- **Effet** : ≥3 refus cumulés → annonce publique de lâcheté. Compteur reset après invocation.
- **Accès** : tous

### `/coalition`
- **Description** : Rejoint la coalition contre un joueur (500c, devient active à 3 membres).
- **Options** : `cible` (User)
- **Effet** : 500c par membre. Activation à 3+ : la cible voit ses gains réduits à 80 % pendant 48h, jusqu'à ce qu'elle batte un membre en duel.
- **Accès** : tous

### `/contribuer-prime`
- **Description** : Ajoute des coins à la prime collective d'un joueur en série.
- **Options** : `cible` (User), `montant` (Integer ≥50)
- **Effet** : Une bounty s'ouvre automatiquement sur quiconque atteint 5 victoires consécutives. Quiconque le bat empoche le pot total (+ titre **Régicide**).
- **Accès** : tous

### `/prime`
- **Description** : Place une prime sur la tête d'un joueur.
- **Options** : `cible` (User), `montant` (Integer ≥1)
- **Effet** : Quiconque bat la cible gagne le pot. Cumulable avec d'autres primes sur la même cible. Visible publiquement.
- **Accès** : tous

---

## 🔒 Prison

### `/travaux`
- **Description** : Effectue une tâche de prison (uniquement disponible en cellule).
- **Effet** : Disponible **uniquement en prison** (post-braquage raté). 50/50 succès. Tâches aléatoires (nettoyer / cuisiner / informer). Réussite : 50-100c + 5 XP. Échec : zéro. Cooldown 2h. Récupération max ~500c sur 24h.
- **Accès** : tous (mais conditionné à la prison)

---

## ⚙️ Configuration

### `/no-taunts`
- **Description** : Active/désactive les railleries automatiques te concernant.
- **Options** : `etat` (`on` | `off`)
- **Effet** : Opt-out des taunts (faillite, jackpot, don généreux). Aucun coût. Réservé à soi-même.
- **Accès** : tous (sur soi)

### `/taunts-channel` 🔒
- **Description** : (Admin) Configure le salon des railleries automatiques.
- **Options** : `salon` (Channel, optionnel — omettre = désactiver)
- **Effet** : Définit où les taunts/renames sont postés. Nécessite que le bot ait la permission **Gérer les pseudos**.
- **Accès** : admin (`MANAGE_GUILD`)

### `/progression-resync` 🔒
- **Description** : Force la vérification des rôles de niveau (texte / vocal / jours).
- **Options** : sub-commands
  - `user @cible` — re-vérifie un membre précis.
  - `me` — re-vérifie soi-même.
  - `all [limit]` — re-vérifie les top N joueurs (défaut 50, max 200, throttle 250 ms).
- **Effet** : Réapplique les rôles XP en lisant l'état actuel. Utile si nouveau reward ajouté à posteriori, changement de mode `xp_role_mode`, ou attribution Discord ratée historiquement.
- **Accès** : admin (`MANAGE_GUILD`)

---

## 🎲 Bonus — La Roue du Destin

> 🪙 **Note** : `/roue` (la signature) est gérée par un module dédié hors `coude`. Voir la documentation correspondante dans `docs/amélioration/COUPE_AMELIORATIONS.md` § 6.0.

---

## 📌 Conventions

- 🔒 = nécessite une permission Discord (`MANAGE_GUILD` typiquement).
- Toutes les autres commandes sont accessibles à tous les membres du serveur.
- Les **commandes payantes** alimentent souvent la **cagnotte communautaire**.
- Les **commandes secrètes** (boost voleur, protection, malédiction…) sont **éphémères** côté Discord — la cible n'est pas notifiée tout de suite.

---

*Document à jour au 2026-04-26 — synchronisé avec la branche `main`. Pour la philosophie du jeu, voir `COUP_DE_COUDE_BUT_DU_JEU.md`.*
