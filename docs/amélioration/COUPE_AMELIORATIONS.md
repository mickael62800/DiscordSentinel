# 🚀 Coup de Coude — Propositions d'améliorations

> Document compagnon de [`COUPE.MD`](./COUPE.MD) qui identifiait 5 faiblesses.
> Ici on propose des **idées concrètes** pour transformer un bon jeu en
> expérience addictive.

## 🧭 Matrice de priorité

| Niveau | Effort | Impact | Quand |
|---|---|---|---|
| 🟢 **Quick wins** | 1-2 jours | Fort | Semaine 1 |
| 🟡 **Moyens** | 1 semaine | Fort | Semaine 2-3 |
| 🔴 **Gros** | 2-4 semaines | Très fort | Mois 2 |

---

## 1. 😵 Trop de complexité perçue

### Problème
Le joueur débutant voit trop de commandes, trop d'items, trop de règles.
Il abandonne avant de comprendre. « Je clique → je comprends → je rejoue »
n'est pas encore vrai.

### 1.1 🟢 Tutoriel interactif en 5 étapes ⏳

- Au premier `/profil` : le bot propose un mini-parcours guidé.
- Étape 1 : choisir une classe (avec résumé 1 phrase de chaque).
- Étape 2 : combat contre un PNJ d'entraînement (le bot) pour comprendre
  les rounds sans risquer de coins.
- Étape 3 : faire un don de 50 coins au bot pour débloquer 150 coins de
  bonus (récompense tutoriel).
- Étape 4 : viser un objet au shop (gratuit la première fois).
- Étape 5 : parier sur un vrai combat en cours.

**Bénéfice** : onboarding naturel, zéro doc à lire, le joueur "sent" les
mécaniques avant de risquer quoi que ce soit.

### 1.2 🟢 Mise par défaut intelligente ✅

- Si le joueur tape `/coude @cible` sans mise, le bot suggère automatiquement
  20 % de ses coins (dans la fourchette min/max serveur).
- Boutons rapides : `50c` / `100c` / `all-in` / `annuler`.
- Supprime le frottement du « combien je mise » pour les joueurs nouveaux
  ou pressés.

### 1.3 🟡 Commande `/aide` contextuelle ✅

Une seule commande qui répond : **« Qu'est-ce que je peux faire maintenant ? »**

Affiche dynamiquement selon l'état du joueur :
- HP bas → "tape `/repos` ou une potion"
- Solde plein + assurance expirée → "pense à `/assurance` avant un gros combat"
- Points de stat non dépensés → "`/train` pour les placer"
- Jamais combattu → "tente un combat facile : `/coude @bot_dummy 20`"

**Bénéfice** : zéro risque de perdre le joueur dans la doc, chaque session
démarre avec une suggestion claire.

---

## 2. 🎆 Manque de spectacle

### Problème
Les combats sont surtout des calculs. Il manque des moments « OH SHIT »
qui font réagir le chat et créent des anecdotes racontées la semaine suivante.

> ⚡ **Ton** : on assume le ridicule. Le jeu est déjà chaotique, on pousse
> le cran **« c'est un peu n'importe quoi et c'est exactement pour ça
> qu'on aime »**. Pas de narration Shakespeare — du Kaamelott dopé aux RedBull.

### 2.1 🟢 Événements chaos **Mythiques** (absurdes assumés) ✅

Ajouter une dizaine d'events à très basse probabilité, tous ridicules,
tous annoncés avec ping serveur :

| Event | Proba | Effet |
|---|---|---|
| 🦄 **Licorne rose** | 0.05 % | Match nul forcé + 500c bonus aux deux. Le salon reçoit un GIF de licorne. |
| 🌠 **Étoile filante** | 0.2 % | Les deux joueurs ressuscitent à 100 %. Round final sudden death. |
| 🎰 **Jackpot divin** | 0.1 % | Le gagnant touche **10× la mise** sortie de la cagnotte serveur. |
| 💀 **Revanche d'outre-tombe** | 0.3 % | Le perdant ressuscite, vole 30 % des coins du gagnant, repart. |
| 🐔 **Invasion de poulets** | 0.2 % | 50 poulets sauvages entrent dans l'arène. Le combat s'arrête. Match nul. |
| 🧻 **Distributeur de PQ** | 0.15 % | Tout le pot devient du PQ. Personne ne gagne rien. Un message humiliant ping les deux. |
| 🍀 **Trèfle à 4 feuilles** | 0.5 % | Le perdant récupère 150 % de sa mise au lieu d'en perdre. Le gagnant est quand même loggé gagnant. |
| 🛸 **Aliens** | 0.05 % | Les deux joueurs sont abducté. Le combat est marqué "non résolu" pendant 24 h. Revient plus tard avec résultat mystère. |
| 🎩 **Le Magicien** | 0.1 % | Les classes des deux joueurs sont échangées **pour ce combat uniquement**. |
| 💣 **Bombe nucléaire** | 0.02 % | Annihilation totale. Les deux joueurs perdent 50 % de leur wallet dans la cagnotte. Message apocalyptique. |

**Bénéfice** : quand un de ces trucs tombe, tout le serveur en parle
pendant 3 jours. La **rareté crée la légende**. « J'étais là quand la
bombe nucléaire est tombée sur le combat de Bob » devient un meme interne.

### 2.2 🟢 Commentaires de combat débiles ✅

Le bot intercale des phrases ridicules pendant le récit du combat :

- « Bob trébuche sur une écharde émotionnelle »
- « Alice refuse de se battre tant qu'elle n'a pas fini son café »
- « Charlie utilise l'attaque spéciale **Mon père est avocat** — aucun effet »
- « Dave hurle une citation de Confucius mal traduite — tout le monde est confus »
- « Eve sort une banane de sa poche, ça n'aide pas »
- « L'arbitre fantôme siffle un hors-jeu inexistant »

Ces lignes sont glissées **aléatoirement** entre les rounds (~20 % chance
par round). Aucun impact mécanique — c'est **du bruit de fond comique**.

**Bénéfice** : transforme un combat calculé en **scène de théâtre absurde**.
Les captures d'écran circulent.

### 2.3 🟢 Détection Clutch / Comeback / Ridicule ✅

Le moteur calcule déjà les HP finaux. On détecte automatiquement :

- **CLUTCH** 🔥 : gagne sous 10 % HP → embed doré + "LE BOB A SURVÉCU !!!"
- **COMEBACK** ⚡ : sous 20 % HP pendant ≥ 2 rounds → badge + annonce
- **PERFECT** 💎 : finit > 90 % HP → "Tu l'as pas vraiment fait transpirer"
- **RIDICULE** 🤡 : combat fini en 1 round sur un 1 au d20 → ping humiliation
- **ZÉRO POINTÉ** 🪦 : les deux finissent à 0 HP en même temps → stat "morts en double"
- **POTO-ÉTAT** 🍻 : ton ami perd un combat sous ton parie → +50 c de consolation
- **KING SIZE** 👑 : tu gagnes 3 combats d'affilée contre le même joueur → lui est renommé « Punching Ball de @toi »

Tout ça stacké dans le profil : *"12 clutchs, 4 KO ridicules, 1 zéro pointé"*.

**Bénéfice** : chaque combat, même banal, a un **potentiel de collector**.

### 2.4 🟡 Narration débridée (pas sage) ✅

Le texte du combat n'est plus informatif, il est **narratif ET méchant** :

Avant :
> « Bob inflige 32 dégâts. Alice inflige 18 dégâts. »

Après :
> « Bob met un coup de boule légendaire — 32 dégâts. Alice le regarde
> comme si elle regardait un clochard à la sortie d'Auchan — 18 dégâts
> de mépris. »

3-5 variantes par situation, piochées au hasard. Incluent :
- **Insultes créatives** (mais familiales)
- **Références absurdes** (mangas / mèmes / actualité décalée)
- **Descriptions exagérées** (un coup de poing = un missile nucléaire)
- **Commentaires méta** (« Le moteur de combat trouve ça louche »)

Pack de **~200 phrases** à rédiger une fois, piochées à vie.

**Bénéfice** : le combat n'est plus un tableau Excel, c'est une **vanne
longue de 400 caractères**.

### 2.5 🟡 Spectateurs fictifs (faux chat stream) ✅

À la fin d'un combat, le bot poste **3-5 faux commentaires de spectateurs
inventés** comme sur un stream Twitch :

```
💬 [Kevin_Le_Hooligan] : MDRRRR j'avais misé sur Alice 😭
💬 [LaReine_du_Troll]  : Bob c'est un clodo on le savait
💬 [Max_69_420]        : KEKW
💬 [GrandMama]         : j'ai gagné 40 coins merci
💬 [Sheep_Lv_1]        : je retire tout
```

Les noms sont pris d'une liste fixe de ~100 pseudos absurdes (Kevin, Jean-Mi,
GrandMama, Sheep_Lv_1, etc.). Les phrases aussi. **Zéro mécanique derrière**,
juste de l'ambiance.

**Bénéfice** : donne l'illusion qu'il y a 200 personnes qui regardent
chaque combat. Rend les petits combats **aussi épiques que les gros**.

### 2.6 🔴 Combat animé round par round ✅

Au lieu de tout poster d'un coup, éditer le message progressivement :
round 1 → pause 2s → round 2 → pause 2s → … → résultat.

Les spectateurs suivent en direct. Sensation de combat streamé.

**Bénéfice** : énorme gain de tension. Réserver aux combats à grosse mise
(> 500c) pour pas ralentir les petits.

---

## 3. 📈 Progression peu excitante

### Problème
Montée de niveau linéaire, +3 points de stats à distribuer, changement
de titre. Ça marche jusqu'au niveau 10, après le joueur décroche.

### 3.1 🟡 Ultimate par classe (débloqué au niveau 10) — versions barrées ✅

Oublie les « +15 % ATK ». Les ultimates doivent faire **halluciner le chat**.
Une par classe, utilisable 1× par semaine via `/ultimate`, thématique mais débile :

- **Bourrin** — 🔄 **Échange de carcasses**
  Tu **swap ton HP courant avec celui de l'adversaire** juste avant le
  combat. Mourant à 5 HP ? L'autre hérite de tes 5 HP et toi tu récupères
  ses 180. Le meilleur outil anti-injuste jamais conçu.

- **Agile** — 🪙 **Pile ou face**
  Le combat est **instantanément résolu** sur un 50/50 pur. Ignore classes,
  niveaux, items, HP, tout. Juste un coin flip. Version premium : tu as
  un buff secret +5 % sur le flip si tu as l'ultimate.

- **Fourbe** — 🏃 **Le Fuyard** (aka "J'ai oublié de payer mon loyer")
  Tu **voles la mise AVANT le combat** et tu te barres. Le défenseur
  reçoit un message « ton adversaire a fui avec la caisse ». Cooldown
  2 semaines (vs 1 pour les autres) parce que c'est abusé.

- **Tank** — 🧱 **Statue**
  Pendant ton prochain combat, tu **ne fais aucun dégât** mais tu
  **prends aucun dégât non plus**. Tu gagnes automatiquement au bout
  de 10 rounds par forfait de l'adversaire qui s'ennuie. Message du
  combat : « Bob n'a pas bougé. Alice est partie chercher un sandwich. »

### 3.1bis 🟡 Ultimates communes débloquables (toutes classes) ⏳

En plus de l'ultimate de classe, le joueur peut acheter des **ultimates
universelles** au shop (très cher, usage unique) :

- 🎲 **Dé chargé** (2 000c) — Tous tes rolls du prochain combat sont des
  20. Sauf un round où tu roll 1 pour la drama.
- 🤡 **Clown mode** (800c) — Pour ce combat, les deux joueurs portent
  un nez de clown. Aucun effet gameplay. Juste cosmétique. Humiliation.
- 🌀 **Rembobine** (5 000c) — Annule le dernier combat perdu, récupère
  les coins. Le journal affiche « truqué » à côté. Saison limitée.
- 🫡 **Déclaration de guerre** (500c) — Ton prochain combat contre une
  cible : elle **ne peut pas refuser** (plus fort que surprise, ça bypass
  même explosion). Mais tu t'engages à jouer **nu** (pas d'items,
  pas d'assurance).
- 🎁 **Cadeau empoisonné** (1 200c) — Envoie un "cadeau" à un joueur.
  Il peut l'ouvrir... ça peut être 1 000c ou -500c. 50/50. Le bot ping.

**Bénéfice** : sensation nette de *"je gagne des pouvoirs en montant"*
ET de *"ce jeu est complètement taré"*. Identité renforcée.

### 3.2 🟡 Paliers visibles (milestones) ✅

Tous les 5 niveaux (5 / 10 / 15 / 20 / 25), débloquer un **effet permanent** :

- Niveau 5 : +1 emplacement d'assurance (cumul de 2 actives).
- Niveau 10 : ultimate de classe (cf 3.1).
- Niveau 15 : réduction du cooldown `/repos` à 8 h (au lieu de 12 h).
- Niveau 20 : pouvoir riposter en premier dans les combats vs joueur < lv.
- Niveau 25 : accès au **Prestige** (cf 3.3).

**Bénéfice** : chaque palier est une **carotte claire**, visible dans le profil.

### 3.3 🔴 Système de Prestige (niveau 25+) ✅

Au lieu de « max level », permettre de **Prestige** :
- Reset au niveau 1 mais **+5 % de gains permanents** par prestige (cumul).
- Affiche des étoiles à côté du pseudo (⭐ / ⭐⭐ / ⭐⭐⭐).
- Débloque un **cosmétique unique** : couleur custom de profil, badge permanent,
  emoji exclusif.
- Cap à 5 prestiges (= +25 % gains perma + 5 étoiles).

**Bénéfice** : les hardcores ont un objectif long terme. Les joueurs moyens
voient le prestige comme un rêve atteignable.

### 3.4 🟢 Achievements cosmétiques ✅

30-50 succès collectables, trackés automatiquement :
- "Survivant" : gagner un combat sous 5 % HP (×1)
- "Giant Killer" : battre un adversaire 5 niveaux au-dessus (×3)
- "Serial Voleur" : réussir 10 vols (×10)
- "Roi du Chaos" : déclencher 20 events chaos différents
- "Millionnaire" : atteindre 100 000 coins
- "Patrouilleur" : bloquer 10 vols avec des items actifs

Chaque succès = un badge dans le profil, aucun avantage gameplay.

**Bénéfice** : les joueurs complétionnistes ont de quoi s'occuper sans
impacter l'équilibre.

---

## 4. 😤 Frustration élevée

### Problème
Le jeu punit : coins perdus, prison 24 h, cowardice, scam d'assurance.
Combiné au RNG, certains joueurs quittent après une grosse défaite.

### 4.1 🟢 Bouclier « malchance du jour » ✅

La **première défaite de la journée** est adoucie automatiquement :
- Perte de coins × 0.5 (au lieu du calcul normal).
- Pas de reset de win streak.
- Message UI : "💔 Mauvaise journée ? Ta prochaine défaite du jour comptera
  normalement."

**Bénéfice** : élimine la spirale "j'ai perdu une fois, je quitte".
Le joueur peut retenter sans risque fatal.

### 4.2 🟢 Refus = pas de lâcheté si HP bas ✅

Si le défenseur est à **< 20 % HP**, refuser un combat **n'incrémente pas**
`cowardice_count`. Message : « Ta cible est trop faible pour se battre ».

Actuellement le joueur est forcé de perdre OU d'être flag lâche —
piège RNG pénalisant. Donne une échappatoire légitime.

### 4.3 🟡 Travaux communautaires en prison ✅

Pendant les 24 h de prison post-braquage raté, au lieu d'être muet :
- Commandes passives restent actives : `/travaux` remplace `/coude`.
- Choix d'une tâche : "nettoyer les cellules" / "cuisiner pour les gardes" /
  "informer la police".
- Gain : 50-100 coins + 5 XP par tâche, cooldown 2 h.
- Chaque tâche = un mini-jeu texte (choix de 3 options random, 50 % succès).
- Total max sur 24 h : environ 500 coins (récup' partielle du coût).

**Bénéfice** : transforme la prison d'une punition stérile en **contenu
de gameplay**. Le joueur reste engagé.

### 4.4 🟡 Filet de sécurité coins ✅

Si le solde du joueur tombe sous **50 coins**, activation automatique de
bonus de survie pendant 3 jours :
- Toutes les pertes sont × 0.5.
- Les paris gagnants × 1.5.
- Un message quotidien : « 💚 Tu es dans la phase de récupération. »

Logique : éviter qu'un joueur pauvre reste pauvre à cause du RNG.

### 4.5 🔴 Mode « duel amical » ✅ *(MVP — leaderboard separe a venir)*

Nouvelle variante : `/coude-amical @cible` — combat **sans mise** ni
conséquence coins. Le gagnant gagne juste **+20 XP** (au lieu de 15) et
les stats apparaissent dans un leaderboard séparé.

Permet d'essayer une classe / un item sans risque.

**Bénéfice** : réduit drastiquement la barrière à l'engagement,
surtout pour les nouveaux joueurs ou pour tester avant un "vrai" combat.

---

## 5. 🤝 Interactions **connardes entre potes** (le vrai manque)

### Problème
Le jeu a du vol et du braquage, mais il **n'exploite pas assez** le fait
que c'est un jeu entre **potes d'un même serveur Discord**. Un Discord,
c'est des amis qui se vannent. Le jeu devrait **amplifier** ça — pas le
contourner.

> 🔥 C'est **le** chaînon manquant. C'est ce qui différencie un bot de
> combat générique d'un jeu *"on s'est marrés tout le week-end"*.

### 5.1 🟢 Malédictions (`/maudire @pote`) ✅

Pose un sort minable sur un ami pendant 24 h. Coût : 300c. Les
malédictions sont **ridicules mais visibles** :

- 🐔 **Malédiction du poulet** : son pseudo est renommé « @Bob le Poulet »
- 🍌 **Peau de banane** : 30 % de chance de rate ses d20 (relance à 1)
- 💸 **Portefeuille troué** : toutes ses transactions coûtent 10c de frais
- 🐌 **Lenteur** : ses messages de combat s'affichent avec 10 secondes
  de retard. Nouveaux joueurs : il croit que le bot bug.
- 🧛 **Insomnie** : les taunts de défaite le concernent +50 %
- 💔 **Malchance amoureuse** : son prochain combat, la licorne ne peut
  PAS tomber (drama si elle allait tomber)

Le maudit voit son état dans `/profil`. **Cumul interdit** (une seule
malédiction active par cible). Mini-jeu annexe pour lever la malédiction
en payant double à son auteur.

**Bénéfice** : permet de troller un pote sans le flinguer financièrement.
Rigolade pure.

### 5.2 🟡 Sabotage (actif) ✅

Pay-to-troll : des actions payantes qui pénalisent un joueur spécifique
sans vol direct de coins.

- **Graisser les armes** (200c) : la prochaine attaque spéciale de @cible
  foire automatiquement.
- **Empoisonner le wallet** (400c) : sur les 3 prochains gains de @cible,
  10 % va à toi.
- **Fausse assurance** (500c) : tu vends à @cible une "assurance" qui se
  révèle être un scam garanti (il le découvre à la perte). Coûte 200c
  en plus de la mise du pote.
- **Coller une pancarte** (150c) : ton pseudo est affiché sous son profil
  pendant 7 jours comme « Rival officiel ». Il ne peut pas l'enlever.

**Bénéfice** : donne des outils d'**agression créative** sans casser
l'économie. Les sabotages sont visibles, personne ne se fait avoir en
silence → la victime **peut répliquer** (revenge).

### 5.3 🟡 Revenge mechanics (vendetta) ✅

Quand tu te fais humilier, tu dois pouvoir **rendre la pareille**.

- **Vendetta officielle** : après avoir perdu contre X, tu peux lancer
  `/vendetta @X`. Dans les 7 jours, si tu gagnes la revanche, tu reçois
  **+100 % de la mise** (au lieu du calcul normal). Si tu perds encore,
  X est renommé « @X le Bourreau de @toi » pour 7 jours.

- **Prime collective** : si un joueur gagne 5 combats d'affilée, une
  prime automatique de 1 000c apparaît sur sa tête. Tout le monde peut
  contribuer. Celui qui le bat rafle le tout + un titre *« Régicide »*.

- **Dette d'honneur** : si un joueur refuse 3 fois un combat contre toi
  → il te *doit* un combat. Tu as 48 h pour lancer `/honneur @lâche` et
  il ne peut pas refuser.

- **Coalition** (advanced) : 3+ joueurs se liguent via `/coalition @cible`.
  Chacun paie 500c. La cible subit -20 % sur tous ses gains pendant 48 h,
  jusqu'à ce qu'elle batte **l'un des conspirateurs** en combat direct.

**Bénéfice** : le jeu devient **une guerre de clans** vivante. Les
rancœurs qui traversent la semaine alimentent les sessions.

### 5.4 🟢 Pranks communautaires ✅

Outils de troll pur, zéro gameplay derrière, juste de l'ambiance :

- 🚨 **Fausse alerte braquage** (100c) — le bot ping @everyone *"BRAQUAGE
  EN COURS, la cagnotte est à XXXc !"* Personne n'est vraiment attaqué.
  Tous s'affolent pour rien.

- 📰 **Faux scoop** (200c) — le bot annonce une rumeur crédible type
  *"Bob vient de perdre 50 000 coins en voulant tout miser"*. Personne
  n'a rien perdu, c'est juste un faux titre de journal.

- 🎭 **Costume obligatoire** (300c) — pendant 24 h, le bot préfixe tous
  les messages de @cible avec un emoji choisi par toi (🤡, 🐷, 🧻, etc.).

- 📞 **Faux appel** (50c) — envoie un DM automatique à un pote comme
  si c'était le bot : *"Tu as gagné 10 000 coins ! Réclame avec /claim"*.
  `/claim` existe pas, ça lui dit *"Trolled"*.

**Bénéfice** : outils à 0 risque financier mais 100 % ambiance.

---

## 6. 🎭 Identité floue → besoin d'un **moment signature**

### Problème
Combat + économie + hasard sans émotion dominante claire. Il manque
**LE truc** que les gens associent instantanément au jeu — le
*"ah ouais, c'est le jeu où..."*.

> 🏆 La signature, c'est **une mécanique unique, ridicule, racontable
> en une phrase**. Pas deux. Pas trois. **Une**.

### 6.0 🟢 🪙 LA SIGNATURE : **LA ROUE DU DESTIN** (une fois par jour) ✅

**Commande unique : `/roue`**.

1× par jour par joueur, tu **spin** la roue. Le bot affiche une animation
de roue qui tourne pendant 5 secondes puis s'arrête sur une case au hasard.

**20 cases**, toutes débiles, toutes mémorables :

| # | Case | Effet |
|---|---|---|
| 1 | 🎰 **Jackpot** | +5 000 coins |
| 2 | 💀 **Ruine** | -500 coins |
| 3 | 🤡 **Clown** | Ton pseudo affiche 🤡 pendant 24 h |
| 4 | 🦁 **Roi du monde** | +titre "Chosen One" pendant 7 jours |
| 5 | 🐍 **Mue** | Tu changes de classe pour 24 h (random) |
| 6 | 💋 **Bisou** | Tu envoies un bisou à un joueur random (le bot ping) |
| 7 | 🥋 **Ceinture noire** | Prochain combat : +50 % dégâts |
| 8 | 🩳 **Slip** | Prochain combat : -50 % dégâts, tu combats "en slip" (mention dans l'embed) |
| 9 | 📦 **Colis** | 3 items aléatoires du shop gratuit |
| 10 | 🧙 **Sorcier** | Prochaine malédiction que tu poses = gratuite |
| 11 | 🛸 **Enlèvement** | Tu es "pris par les aliens" — tu ne peux pas jouer pendant 1 h |
| 12 | 🎁 **Père Noël** | 200c donnés à un joueur random du serveur |
| 13 | 🌀 **Blanche** | Rien. Rien du tout. Retry demain. |
| 14 | 🧻 **PQ** | Tu reçois 1 000 rouleaux de PQ. Aucun usage. Pure collection. |
| 15 | 🔥 **Mode hardcore** | Prochain combat : tu gagnes double, tu perds double |
| 16 | 💤 **Sieste** | +20 % régen HP pendant 12 h |
| 17 | 👑 **Couronne** | Tu deviens "Roi du jour" — annonce serveur, effet cosmétique 24 h |
| 18 | 🎪 **Cirque** | Tu organises un "cirque" : prochaine commande `/coude` offre 100c aux 5 premiers spectateurs qui parient |
| 19 | 🔄 **Swap** | Tu échanges ton wallet avec un joueur **choisi au hasard par le bot** (si il a moins de 1 000c pour éviter les abus) |
| 20 | 🦄 **LICORNE** | 1 % de chance sur les 5 % ultra-rare : +10 000c + annonce @everyone |

Les probas peuvent être tweakées (cases 1 / 19 / 20 plus rares).

**Pourquoi c'est LA signature :**

✅ **Tout le serveur la voit** : la roue est publique (postée dans un
  salon dédié). Tout le monde voit ce que tombe à chaque spin.
✅ **Racontable en une phrase** : « C'est le jeu où tu spin une roue
  débile chaque jour ». Tout de suite compris.
✅ **Crée un rituel quotidien** : les gens se connectent juste pour
  voir ce qu'ils tirent, même sans combattre. **Rétention garantie**.
✅ **Totalement ridicule** : PQ, slip, enlèvement alien — le ton du
  jeu est assumé.
✅ **Fonctionne avec le reste** : chaque case renforce les autres
  mécaniques (combat, classe, malédiction, items).
✅ **Viral par nature** : les captures des résultats débiles circulent
  hors serveur.

**Tagline associée** : *« Coup de Coude — le jeu Discord où t'as pas
besoin de jouer pour que la journée soit déjà pétée. »*

### 6.1 🟡 TOUT-OU-RIEN (backup signature, si Roue du Destin trop complexe) ✅

Commande `/tout-ou-rien` — disponible 1× par semaine :

1. Tu mises **l'intégralité de ton wallet**.
2. Le bot roll un 50/50 avec animation 10 secondes.
3. **Pile** : tu doubles ton solde. Annonce serveur avec crown.
4. **Face** : tu perds **80 %** de ton wallet. Le bot t'ajoute au
   "Memorial des clodos" — leaderboard public des plus gros suicides.

Le Memorial est un leaderboard auquel on s'abonne sans le vouloir.
Tout le monde en parle. Les potes se chambrent.

**Pourquoi c'est viral** : l'idée d'un all-in ridicule, les gens
**parient** entre eux sur qui va tenter cette semaine. Les records
circulent (« Bob a perdu 47 000 coins au tout-ou-rien mdr »).

### 6.2 🟡 Moment de la semaine (highlight reel) ✅

Chaque dimanche à 23 h, un bot poste automatiquement dans le salon principal :

```
🏆 MOMENT DE LA SEMAINE — semaine du 13-20 avril

⚔️ Combat le plus épique : Bob vs Alice (7 rounds, 3 chaos events)
🔥 Comeback de la semaine : Charlie (0.5 % HP → victoire)
💀 KO le plus rapide : Dave (1 round, critique sauvage)
🎰 Plus gros jackpot : Eve (12 400 coins volés à Frank)
🌠 Event légendaire : @Bob a déclenché une Étoile Filante !
```

Automatique, basé sur les stats déjà loggées.

**Bénéfice** : crée une **tradition hebdo** que les joueurs attendent.
Récompense les exploits même sans gain tangible. C'est la machine à
souvenirs partagés qui forge l'identité.

### 6.3 🟡 Saisons thématiques ✅

Chaque saison (90 jours actuels) a un **thème** annoncé :
- **Saison du Chaos** : events chaos ×2 cette saison.
- **Saison du Tank** : +20 % DEF pour les Tanks.
- **Saison du Vol** : gains de vol ×1.5, mais protections -25 %.
- **Saison du Braquage** : cooldown braquage divisé par 2.

Modifie l'équilibre sans toucher au code, juste via config. Les joueurs
**s'adaptent** à chaque saison.

**Bénéfice** : le jeu "respire", chaque 3 mois apporte un twist. Renforce
l'**identité : un jeu vivant qui bouge**.

### 6.4 🟢 Tagline + ton des embeds ✅

Choisir **une** identité et l'assumer partout dans les textes :

> « **Coup de Coude — Chaque duel raconte une histoire.** »

OU

> « **Coup de Coude — Le jeu Discord où le chaos gagne toujours.** »

OU

> « **Coup de Coude — Pari, vole, combats. Surtout : survis.** »

À afficher dans `/aide`, dans les embeds de combat, dans le footer des
messages. Cohérence = identité.

### 6.5 🔴 Événement serveur live (1× par mois) ✅

Un samedi soir par mois, **Grand Tournoi Live** :
- Annonce 3 jours avant.
- Inscription via `/tournoi join`.
- Combats en bracket éliminatoire, animés par le bot en direct.
- Le gagnant récupère 50 % de la cagnotte du mois + titre permanent.

C'est l'événement **qui rassemble** le serveur autour du jeu.

**Bénéfice** : passe de « jeu individuel » à « jeu communautaire ».
Effet viral : les participants invitent leurs potes.

---

## 📅 État d'avancement (mis à jour 2026-04-26)

### ✅ Livré (le gros)
- **`/aide` contextuelle** (1.3)
- **Mise par défaut intelligente** (1.2) — pick UI 20%/50c/100c/all-in si mise omise
- **Événements chaos Mythiques** (2.1)
- **Commentaires de combat débiles** (2.2)
- **Clutch / Comeback / Ridicule** (2.3)
- **Narration débridée** (2.4)
- **Spectateurs fictifs** (2.5)
- **Combat animé round par round** (2.6)
- **Ultimates de classe** (3.1) — Bourrin, Agile, Fourbe, Tank
- **Paliers milestones** (3.2) — incl. niveau 5 slot assurance +1
- **Système de Prestige** (3.3) — incl. wiring multiplicateur sur payouts
- **Achievements cosmétiques** (3.4)
- **Bouclier malchance du jour** (4.1) — 1ʳᵉ défeat ×0.5 + win streak preservée
- **Refus OK si HP bas** (4.2)
- **Travaux communautaires en prison** (4.3)
- **Filet de sécurité coins** (4.4)
- **Mode duel amical** (4.5) — `/coude-amical` MVP (leaderboard séparé en suivi)
- **Malédictions `/maudire`** (5.1)
- **Sabotage** (5.2) — graisser, empoisonner wallet, fausse assurance
- **Revenge / Vendetta** (5.3) — vendetta, prime collective, dette d'honneur, coalition
- **Pranks communautaires** (5.4)
- **🪙 La Roue du Destin** (6.0) ⭐ signature
- **TOUT-OU-RIEN + Memorial** (6.1)
- **Moment de la semaine** (6.2)
- **Saisons thématiques** (6.3)
- **Tagline officielle propagée aux footers** (6.4) — `COUDE_TAGLINE_SHORT`
- **Tournoi mensuel live** (6.5)

### ⚙️ Infra — Configuration par-guild (migration 170)
13 paramètres exposés au schema de `coude-bot` (modifiables depuis la web admin `/components/config`) :
- `mise_pick_suggested_percent` (1.2)
- `lucky_shield_enabled`, `lucky_shield_loss_percent` (4.1)
- `assurance_extra_slot_level` (3.2)
- `prestige_unlock_level`, `prestige_max_count`, `prestige_gain_bonus_percent` (3.3)
- `friendly_winner_xp`, `friendly_loser_xp` (4.5)
- `safety_net_trigger_coins`, `safety_net_duration_hours`, `safety_net_loss_percent`, `safety_net_bet_gain_percent` (4.4)

Helper `CoudeGuildSettings` (`services/api/src/application/coude_guild_settings.rs`) centralise la lecture. Variantes `_with_multiplier` / `_with_params` ajoutées aux fonctions domaine pour préserver les tests existants.

### ⏳ Reste à faire
- 🟢 **Tutoriel interactif 5 étapes** (1.1) — onboarding nouveaux joueurs
- 🟡 **Ultimates communes au shop** (3.1bis) — Dé chargé, Clown mode, Rembobine, Déclaration de guerre, Cadeau empoisonné
- 🔵 **Leaderboard duel amical** (extension 4.5) — exposer `friendly_wins` / `friendly_losses` dans `/leaderboard`

### 🎯 Verdict
27/28 features livrées + 13 paramètres exposés au panneau admin. Le cœur du jeu — chaos, prestige, paliers, sabotage, vendetta, signature roue, duel amical — est complet et **paramétrable par-guild**. Reste de l'onboarding (tutoriel) et 2 extensions cosmétiques.

---

## 🎯 Métriques à suivre

Pour valider que les changements fonctionnent :

- **Rétention J1 / J7 / J30** : pourcentage de joueurs qui reviennent.
- **Sessions par semaine** par joueur actif.
- **Taux d'abandon post-défaite** : combien quittent après une grosse perte.
- **Messages dans le chat par combat** : mesure le "spectacle".
- **Nouvelles inscriptions** hebdo : mesure la viralité.
- **% de joueurs qui spin la Roue chaque jour** : la signature fonctionne ?
- **Captures d'écran partagées hors serveur** : meilleur indicateur viralité.

Baseline avant changements, re-mesure après chaque vague.

---

## 🧠 Verdict d'ensemble

Le jeu actuel est **80 % fini**. Les 20 % qui manquent ne sont pas du
code — c'est du **ton** :

1. **Faire rire** (spectacle, commentaires débiles, spectateurs fictifs)
2. **Faire gueuler** (chaos mythique, ultimates débridées, sabotage)
3. **Faire raconter** (moment de la semaine, roue du destin)
4. **Faire revenir** (rituel quotidien, filet de sécurité, malédictions)
5. **Faire s'embrouiller** (revenge, coalition, pranks entre potes)

### La ligne directrice

> **Assume le ridicule.**
>
> Un pote qui spin une roue débile, qui se prend une malédiction de
> poulet, qui fuit en volant la mise avant le combat, qui se fait
> saboter par la coalition des autres, qui finit 1er du "Memorial des
> clodos" après un TOUT-OU-RIEN désastreux — **c'est ça le jeu**.
>
> Pas un simulateur de combat. Un **terrain de jeu entre potes** où
> tout peut virer n'importe comment. Le chaos est la feature, pas le bug.

### Test ultime

Quand un joueur raconte le jeu à un nouveau, il doit pouvoir lâcher
**en UNE phrase** :

> *« C'est le jeu Discord où tu spin une roue complètement pétée chaque
> jour, tu te maudis entre potes, et parfois une licorne rose te rend
> riche. »*

Si c'est **ça** qui sort spontanément → signature trouvée.

Si c'est *« c'est un bot de combat avec des stats »* → on a raté.

---

*Document rédigé le 20 avril 2026, complément à `COUPE.MD`.
Post-feedback « push le débile, assume le toxic fun entre potes ».*
