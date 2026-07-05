# ✅ Checklist de test — Toutes les commandes

> Coche ✅ ce qui marche, ❌ ce qui ne marche pas. Teste avec le **bon compte**
> (Owner / Admin / Modérateur / Utilisateur). Les niveaux plus élevés peuvent
> aussi lancer les commandes des niveaux inférieurs.

---

## 👑 OWNER / PROPRIÉTAIRE (toi + co-fondateur)

Pas de commande dédiée, mais **une action à tester** :

- [ ] Ticket **« Problème avec un modérateur »** (via le panneau `/ticket-admin panel` puis bouton) → doit **te ping (owner)** et créer un salon **non visible par les modos**. Les co-fondateurs configurés (`ticket_owner_ids`) y ont accès.

---

## ⚙️ ADMIN

### Configuration / modules
- [ ] `/automod` — configuration de l'automodération
- [ ] `/security` — configuration sécurité (anti-raid, etc.)
- [ ] `/audit` — configuration de l'audit (salons de logs, etc.) *(sous-commandes : channel set…)*
- [ ] `/cleanup` — nettoyage
- [ ] `/purge nombre:<n> [utilisateur] [user_id]` — supprime des messages
- [ ] `/rotation` — rotation (statuts/annonces)
- [ ] `/roles-panel` — déploie le panneau de rôles
- [ ] `/progression-resync` — resynchronise les niveaux

### Jeux — mise en place (admin)
- [ ] `/blackjack-setup` — installe le blackjack
- [ ] `/slot-setup` — installe la machine à sous
- [ ] `/wheel-setup` — installe la roue
- [ ] `/tama-setup` — installe le tamagotchi
- [ ] `/game-admin` — administration du Game Portal (serveurs de jeu Docker)
- [ ] `/taunts-channel` — salon des provocations (Coup de Coude)

---

## 🛡️ MODÉRATEUR

### Sanctions
- [ ] `/warn utilisateur/user_id raison:<…>` — avertissement
- [ ] `/unwarn` — retire un avertissement (DM au membre)
- [ ] `/mute utilisateur raison:<…>` — mute temporaire
- [ ] `/unmute` — lève le mute (DM au membre)
- [ ] `/ban utilisateur raison:<…> [jours 0/1/3/7]` — **ban direct** (scam/raid)
- [ ] `/ban-sursis membre raison:<…>` — **ban en sursis** (rôle Sursis + salon d'appel + délai)
- [ ] `/unban user_id` — débannit
- [ ] `/massban` — ban de masse (plusieurs IDs)
- [ ] `/massmute` — mute de masse

### Salon d'appel (à tester après un /warn ou /ban-sursis)
- [ ] Bouton **« Contester cette sanction »** (dans le DM du sanctionné) → crée un salon sous la catégorie d'appel
- [ ] Bouton **🗳️ Voter : annuler** (modo) → ajoute un vote, compteur X/Y
- [ ] Bouton **✅ Valider l'annulation (admin)** → lève la sanction une fois le quorum atteint
- [ ] Bouton **🔨 Fermer + bannir** → confirmation 2 clics → ban + ferme
- [ ] Bouton **🔒 Fermer le salon**

### Outils modo
- [ ] `/history utilisateur/user_id` — historique de sanctions
- [ ] `/note` — note interne sur un membre
- [ ] `/call` — convoque un membre (salon dédié)
- [ ] `/signalement` — carte de signalement
- [ ] `/context` — contexte d'un message
- [ ] `/copilote` — assistant de modération
- [ ] `/expirations` — sanctions qui expirent
- [ ] `/compare` — comparer des membres
- [ ] `/modstats` — stats de modération
- [ ] `/evidence` — preuves attachées à une action
- [ ] `/review` — revue d'action automod
- [ ] `/template` — modèles de réponse
- [ ] `/transcript` — transcript d'un ticket
- [ ] `/export` — export de données
- [ ] `/ticket-admin` — panneau tickets *(sous-commandes : panel…)*
- [ ] `/confess-admin` — administration des confessions

### Automod (à tester en direct)
- [ ] Poster un message qui déclenche l'automod → **carte de sanction** ; le bouton « bannir » applique un **ban-sursis** (si rôle Sursis configuré)

---

## 👤 UTILISATEUR (tout le monde)

### Tickets & appels
- [ ] `/ticket` — ouvre un ticket de support
- [ ] `/appeal` — conteste une sanction (crée le salon d'appel)
- [ ] `/confess` — confession anonyme

### Progression / niveaux
- [ ] `/level` — ton niveau
- [ ] `/stats` — tes statistiques
- [ ] `/classement` — classement mensuel
- [ ] `/parrain` — parrainage

### 🎮 Coup de Coude
- [ ] `/coude` — combat
- [ ] `/coude-amical` — combat amical
- [ ] `/profil [joueur]` — profil de combat
- [ ] `/shop` — boutique
- [ ] `/leaderboard` — classement Coude
- [ ] `/voler` — voler des coins
- [ ] `/train` — entraînement (points de stat)
- [ ] `/classe` — choisir sa classe
- [ ] `/donner` — donner des coins
- [ ] `/hp` — points de vie
- [ ] `/repos` — récupérer des HP
- [ ] `/potion` — utiliser une potion
- [ ] `/reset-stats` — réinitialiser ses stats
- [ ] `/resume` — reprendre
- [ ] `/no-taunts` — désactiver les provocations
- [ ] `/prank` — farce
- [ ] `/aide` — suggestions contextuelles
- [ ] `/tout-ou-rien` — quitte ou double (1×/semaine)
- [ ] `/memorial` — mémorial des clodos
- [ ] `/cagnotte` — caisse communautaire

### 🎰 Casino / jeux
- [ ] `/blackjack` — jouer au blackjack
- [ ] *(slot / roue / tamagotchi : via panneaux/boutons après le setup admin)*
- [ ] `/game` — Game Portal (parcourir / lancer un serveur de jeu)

### 🏛️ Jeu Influence
- [ ] `/influence-profil [joueur]` — profil (capitaux en paliers)
- [ ] `/capital` — tes capitaux exacts + historique
- [ ] `/transfert conversion:<…> montant:<n>` — convertir un capital (coins ↔ …)
- [ ] `/org create type:<…> nom:<…> [devise]` — fonder une organisation (coûte des coins)
- [ ] `/org info nom:<…>` — fiche d'une orga
- [ ] `/org join nom:<…>` — rejoindre une orga (reçoit le rôle si créé)
- [ ] `/org membres nom:<…>` — membres
- [ ] `/org role nom:<…>` — créer le rôle Discord de l'orga (fondateur payant / modo gratuit)
- [ ] `/org relation nom:<…> cible:<…> type:<…>` — alliance / rivalité / boycott (dirigeants)
- [ ] `/vote org:<…> sujet:<…>` — ouvrir un vote d'org (boutons Pour/Contre/Abstention)
- [ ] `/loi propose titre:<…> texte:<…>` — proposer une loi (vote de tous, clôture worker)
- [ ] `/enquete cible:<@> sujet:<…>` — lancer une enquête (payant, résultat différé en DM)
- [ ] `/dossier` — ton intel secret
- [ ] `/reveler info:<id>` — révéler une info → **scandale** (+ presse si activée)
- [ ] `/actu` — fil d'actualité du serveur
- [ ] `/archives` — mémoire du serveur

---

## 🔔 À vérifier aussi (systèmes automatiques, pas des commandes)

- [ ] **Bienvenue / re-bienvenue** à l'arrivée d'un membre
- [ ] **Bump** Disboard **et** DiscordL → coins + rappel après cooldown
- [ ] **Log des commandes admin** (si activé) → une ligne « X a utilisé /commande » + raison
- [ ] **Carte de rôles** : ajoute/retire plusieurs rôles → **une seule carte** qui se met à jour (5 min)
- [ ] **Log de suppression de message** → indique **qui** a supprimé
- [ ] **Agence de presse Influence** (si activée) → scandales / lois / créations d'orga dans le salon presse
- [ ] **Vérification d'âge** au règlement (rôle Membre temporaire → Membre)
- [ ] **Worker** : sursis expiré → ban auto ; loi échue → clôture ; enquête → résultat
