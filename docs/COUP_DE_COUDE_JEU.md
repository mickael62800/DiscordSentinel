# 🥊 Coup de Coude — Guide du joueur

> Bienvenue dans **Coup de Coude**, le mini-jeu de bagarre du serveur ! Gagne des **coins**,
> monte en niveau, défie tes amis, vole-les, équipe-toi et deviens la légende du serveur.
>
> 👉 Toutes les commandes sont listées dans [COUP_DE_COUDE_COMMANDES.md](./COUP_DE_COUDE_COMMANDES.md).

---

## 🚀 Bien démarrer en 4 étapes

1. **Regarde ton profil** avec `/profil`. Tu y vois ton niveau, tes coins, tes PV et tes stats.
2. **Choisis ta classe** avec `/classe` (voir plus bas pour t'aider à choisir).
3. **Lance ton premier combat** avec `/coude` contre un autre joueur, ou entraîne-toi sans rien risquer avec `/coude-amical`.
4. **Bloqué ?** Tape `/aide` : le jeu te suggère quoi faire selon l'état de ton compte.

---

## 📈 Monter en niveau

Tu gagnes de l'**XP** en combattant (et en volant, en braquant…). Chaque montée de niveau te donne des **points de stats** à investir avec `/train` dans :

- **⚔️ Attaque (ATK)** — pour taper plus fort.
- **🛡️ Défense (DEF)** — pour encaisser et avoir plus de PV.

Plus tu montes, plus ton **titre** évolue : Débutant → Bagarreur → Guerrier → Vétéran → Champion → **Inarrêtable** (niveau 25, le max).

> 💡 Astuce : tu peux redistribuer tous tes points avec `/reset-stats` (300c) si tu t'es trompé de build.

---

## 🎭 Les 4 classes

Choisis selon ta façon de jouer :

| Classe | Style | Son atout |
|---|---|---|
| 💪 **Bourrin** | Gros dégâts | Devient enragé (+25 % d'attaque) quand il est presque mort |
| 🏃 **Agile** | Esquive | 15 % de chance d'éviter complètement les dégâts à chaque échange |
| 🗡️ **Fourbe** | Voleur | Se soigne en tapant (vampirisme) et vole mieux que les autres |
| 🛡️ **Tank** | Mur | Réduit tous les dégâts reçus, énormément de PV |

> 💡 Tu peux changer de classe quand tu veux avec `/classe`.

---

## ⚔️ Le combat (`/coude`)

Tu défies un joueur et tu **mises des coins**. Le combat se joue en plusieurs échanges (rounds) : à chaque round, un jet de dé décide des dégâts, modulés par ton attaque, la défense de l'adversaire, ta classe et tes items.

- **Tu gagnes** → tu rafles la mise. **Tu perds** → tu la perds.
- **Égalité** → tout le monde est remboursé.
- Si personne n'est K.O. à la fin, **gagne celui qui a le plus de PV restants**.

Quelques règles utiles :
- Tu ne peux pas combattre si tes PV sont trop bas → soigne-toi d'abord (`/repos`, `/potion`).
- Battre **beaucoup plus fort que toi** rapporte **double XP** (et le plus fort est handicapé).
- ⚠️ Mais battre toujours les mêmes finit par t'attirer des ennuis (vendettas, primes…).

### 🌈 Les événements rares
Très rarement, un combat part en cacahuète : licorne rose qui fait match nul, jackpot qui multiplie la mise par 10, invasion de poulets, bombe nucléaire… C'est rare, c'est annoncé à tout le monde, et ça fait des histoires. 😄

---

## 💰 Gagner (et perdre) des coins

**Pour t'enrichir :**
- Gagner des combats 🥊
- **Voler** les autres (`/voler`)
- **Braquer** la caisse commune (`/braquage`)
- Tenter le **tout-ou-rien** (`/tout-ou-rien`)
- **Parier** sur les combats des autres (`/pari`)

**La caisse communautaire** (`/cagnotte`) est un pot commun alimenté par les paris — c'est la cible des braquages.

**La prime** : si quelqu'un enchaîne **5 victoires d'affilée**, une prime s'ouvre sur sa tête. Le premier qui le bat empoche la prime et le titre de « Régicide ». Tu peux gonfler la prime avec `/contribuer-prime`, ou en poser une toi-même avec `/prime`.

---

## 🥷 Voler & se protéger

- **`/voler`** : tente de piquer une partie du portefeuille d'un joueur. Tu prends plus à quelqu'un qui ne se protège pas. (Cooldown : une fois tous les 7 jours.)
- **`/protection`** : abonne-toi à une protection anti-vol (du simple chien de garde à la forteresse) pour réduire ou bloquer les vols.
- **`/assurance`** : si on te vole, l'assurance limite ta perte. ⚠️ Méfie-toi des fausses assurances (un saboteur peut te piéger).
- **`/boost-voleur`** : pour les voleurs ambitieux, augmente tes chances de réussir tes coups.

---

## 🛒 La boutique (`/shop`)

Trois rayons :

- **Attaque** : Rage (+50 % ATK), Poison, Double Coup, Coup Traître, Surprise (l'adversaire ne peut pas refuser)…
- **Défense / soin** : Potions de soin, Bouclier, Antidote, Explosion (annule un duel)…
- **Braquage** : outils qui augmentent tes chances de réussir un casse.

> 💡 Les items font souvent la différence dans un combat serré. Pense à t'équiper avant un gros duel.

---

## ❤️ Gérer ses PV

- **`/hp`** : voir tes points de vie.
- **`/repos`** : récupère **tous** tes PV (une fois toutes les 12h).
- **`/potion`** : soin rapide hors combat.
- Tes PV remontent aussi tout seuls, doucement, avec le temps.

---

## 🏦 Braquage & prison

- **`/braquage`** : tente de braquer la caisse commune (1×/semaine). Gros gains possibles… mais si tu échoues, **direction la prison** !
- En prison, tu ne peux plus jouer normalement : seule option, **`/travaux`** (petits boulots) pour gagner un peu et passer le temps jusqu'à ta libération.
- Tes outils de braquage s'usent à chaque tentative.

> 💡 Plus tu achètes d'outils de braquage, plus tes chances montent. Sans rien, c'est quasi suicidaire.

---

## 🎲 Tout-ou-rien

**`/tout-ou-rien`** : tu mises **TOUT** ton portefeuille sur un pile ou face (une fois par semaine). Gagné = tu doubles. Perdu = tu ne gardes que des miettes. Pour les courageux (ou les inconscients).

Les plus belles ruines sont immortalisées dans le **`/memorial`** 😂

---

## ⭐ Devenir une légende : Ultimate & Prestige

- **`/ultimate`** (dès le niveau 10) : un pouvoir surpuissant propre à ta classe (échanger ses PV, victoire automatique, 50/50, vol de mise…). Longue recharge, à garder pour le bon moment.
- **`/prestige`** (niveau 25) : tu repars au niveau 1 mais tu **gardes tes coins**, et tu gagnes un **bonus permanent de +5 % sur tous tes gains** (jusqu'à 5 prestiges = +25 %). Affiché en ⭐ sur ton profil.

---

## 🗓️ Les saisons

Le jeu vit par **saisons**. Chaque saison a un **thème** qui change les règles du moment (plus de chaos, tanks renforcés, vols plus juteux, braquages facilités…). Tape `/saison` pour voir le thème en cours et adapte ta stratégie !

---

## 😈 S'amuser (et embêter les autres)

- **`/maudire`** : pose une malédiction ridicule sur un pote (le renommer « le Poulet », lui filer la poisse aux dés, etc.).
- **`/saboter`** : coups bas ciblés (faux rival, arme sabotée, wallet empoisonné…).
- **`/vendetta`** : déclare officiellement la guerre à quelqu'un après une défaite — la revanche rapporte double.
- **`/coalition`** : ligue-toi avec d'autres contre un joueur trop fort.
- **`/no-taunts`** : si les railleries automatiques après combat t'agacent, coupe-les.

---

## 🏆 Suivre les classements

- **`/leaderboard`** : le top des joueurs.
- **`/profil`** : tes stats détaillées (et celles des autres).
- **`/resume`** : tes derniers mouvements de coins.

---

**Bon courage, et que le meilleur gagne !** 🥊
