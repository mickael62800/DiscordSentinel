# 🛋️💥 Coussin Piégé

> Tu planques un coussin sur le canapé. Quelqu'un s'assoit dessus. Le salon rigole.

Le jeu de bagarre de La Bande du Canapé. Il se **joue sur Discord** — les actions n'ont
d'intérêt que devant témoins. Le **site** sert à consulter : ta place, tes bagarres, ton
butin, le classement.

---

## En trente secondes

1. `/classe` — choisis ta manière d'occuper le canapé. Sans elle, tu es Écraseur par défaut.
2. `/coussin @quelqu'un 50` — glisse un coussin sous lui pour 50 coins.
3. Il clique **Accepter** ou **Refuser**. S'il accepte, la bagarre se résout toute seule.
4. Le gagnant empoche la mise. Les deux gagnent de l'XP.

Ton porte-monnaie est le **même** que celui de la Roue du Destin. Un seul solde pour tous
les jeux.

---

## Ce qu'on perd : du Confort

Pas des points de vie. À zéro tu ne meurs pas — **tu te lèves du canapé**. C'est le même
chiffre qu'ailleurs, mais il raconte enfin quelque chose.

Deux statistiques le font bouger :

| Stat | C'est quoi |
|---|---|
| 🧱 **Impact** | Ce que tu fais perdre en face |
| 🪶 **Moelleux** | Ce que tu encaisses — il augmente aussi ton Confort maximum |

---

## Les quatre manières de s'asseoir

| Classe | Impact / Moelleux de départ | Ce qu'elle fait |
|---|---|---|
| 🪑 **Écraseur** | 25 / 8 | Frappe **+25 %**. Et **+25 % de plus** quand il lui reste moins de 30 % de Confort — le baroud d'honneur de celui qui s'écroule. |
| 🤸 **Ressort** | 12 / 18 | Équilibré, monte vite en Moelleux. |
| 🪡 **Piégeur** | 18 / 14 | La seule classe qui **fouille mieux** : 50 % de réussite au lieu de 30 %. |
| 🛌 **Couette** | 8 / 25 | **+30 % de Confort max**, encaisse **20 % de moins**, et retire 5 à chaque coup reçu. |

Changer de classe **remet les statistiques à celles de la nouvelle classe** — les points
dépensés en `/train` sont perdus. À choisir tôt, donc.

---

## La bagarre

`/coussin @membre <mise>` → un embed public avec deux boutons, **Accepter** / **Refuser**.

- **Refuser** est enregistré : ton compteur « fois resté debout » monte, et il se voit sur ton profil.
- **Accepter** déclenche la résolution : jusqu'à **3 manches**, chacune avec un jet de dé
  (1 à 6) pour chaque camp. Celui qui a le plus de Confort à la fin gagne. À égalité de
  Confort, c'est le total de dégâts qui départage ; si tout est égal, match nul.
- Le gagnant prend la mise. Match nul : personne ne paie.

**Écart de niveau.** On ne tape pas un débutant impunément :

| Écart de niveau | Effet |
|---|---|
| 0 à 2 | Rien |
| 3 à 5 | Le plus haut niveau perd 20 % d'Impact |
| 6 à 9 | Il en perd 40 % |
| 10 et plus | **Bagarre refusée** |

**Progression.** Le gagnant prend 15 XP, le perdant 5 — perdre rapporte quand même, sinon
on arrête de jouer. Chaque niveau donne **3 points** à placer avec `/train`. Le niveau
maximum est **25**.

| Niveau | Titre |
|---|---|
| 1–4 | Bout d'Accoudoir |
| 5–9 | Squatteur |
| 10–14 | Poseur de Coussins |
| 15–19 | Gardien de la Télécommande |
| 20–24 | Roi du Canapé |
| 25 | *Le Canapé, c'est Lui* |

---

## Toutes les commandes

### Public — tout le salon voit

| Commande | Ce qu'elle fait |
|---|---|
| `/coussin <membre> <mise>` | Glisse un coussin sous quelqu'un. Il accepte ou refuse. |
| `/chiper <membre>` | Fouille sous les coussins de quelqu'un. |
| `/contrat <membre> <montant>` | Promets une récompense à qui le fera lever. |
| `/pari <combat> <membre> <montant>` | Parie sur une bagarre en cours. |

C'est volontaire : ces quatre-là visent quelqu'un d'autre, ou vivent de la réaction du
salon. Les cacher viderait le jeu de son sel.

### Privé — réponse éphémère, toi seul la vois

| Commande | Ce qu'elle fait |
|---|---|
| `/profil [membre]` | Place sur le canapé : classe, Confort, palmarès. |
| `/classe <classe>` | Choisis ta manière de t'asseoir. |
| `/train <impact\|moelleux>` | Dépense un point gagné en montant de niveau. |
| `/shop <objet>` | Le coffre à coussins. |
| `/inventaire` | Ce que tu planques sous ton coussin. |
| `/garantie` | Garantie anti-tache, 1 heure. |

Éphémère par nécessité, pas par pudeur : annoncer publiquement que tu viens d'acheter une
**Punaise dans le Coussin** ruinerait l'objet avant que tu t'en serves.

### Le porte-monnaie (commun à tous les jeux)

`/solde` · `/donner <membre> <montant> [raison]` · `/classement` · `/roue`

---

## La fouille (`/chiper`)

Tu passes la main sous les coussins de quelqu'un.

- **Réussite** : 30 % — mais **50 % si tu es Piégeur**. Tu prends **20 %** de son solde.
- **Échec** : tu perds **15 %** du tien, et **ça part chez ta cible**. C'est ce qui rend
  la fouille risquée plutôt que gratuite.
- **Délai** : 30 minutes entre deux fouilles.
- **Plancher** : on ne fouille pas quelqu'un sous **10 coins**. Achever un fauché ne
  rapporte rien et le dégoûte du jeu.

Tous ces chiffres sont **réglables par serveur** (voir plus bas).

---

## Le coffre à coussins (`/shop`)

| Objet | Prix | Effet |
|---|---|---|
| 🧱 Coussin Plombé | 100 | +50 d'impact pour une bagarre |
| 👁️ Œil sous le Plaid | 150 | Révèle le jet d'en face |
| 🍟 Renversement de Chips | 200 | Annule le gain de la bagarre |
| 🛋️ Double Coussin | 250 | Garde le meilleur de deux jets |
| 🪶 Bataille d'Oreillers | 300 | Défi immédiat, sans prévenir |
| 📌 Punaise dans le Coussin | 350 | Ignore le moelleux adverse |
| 🔄 Retourne le Canapé | 500 | Échange les soldes |

## La garantie anti-tache (`/garantie`)

50 coins, une heure de couverture sur tes pertes.

**Une garantie sur vingt est une arnaque.** Le bot te le laisse deviner — le contrat a
l'air louche, mais il ne dit pas ce qu'il changera. C'est la blague de la mécanique, et
elle n'est **pas réglable** : à 0 % l'achat n'a plus d'histoire, à 100 % ce n'est plus une
garantie.

## Contrats et paris

- **`/contrat`** — tu mets une somme sur la tête de quelqu'un pour qu'on le fasse lever du
  canapé. Minimum 50 coins. On ne peut pas se mettre un contrat sur soi-même.
- **`/pari`** — tu mises sur une bagarre en cours avec son identifiant. Minimum 10 coins,
  un seul pari par bagarre. Le gain est calculé à la résolution.

---

## Sur le site

**Page publique** (`/jeux`, panneau *Coussin Piégé*) — ta fiche, ton Confort en jauge, ton
palmarès, ce que tu planques, tes dernières bagarres et le classement. En lecture seule :
rejouer les actions depuis un navigateur contournerait les mises, les délais et le
consentement de l'adversaire.

**Back-office** (`/nexus/coussin`) — la table de tous les joueurs d'un serveur. Elle sert à
repérer ce que Discord ne montre pas : quelqu'un qui décroche, un fouilleur en série, une
accumulation anormale de coins.

---

## Réglages par serveur

Page **Configuration**, module **Coussin Piégé** (`nexus-coussin`). Les valeurs par défaut
sont celles indiquées plus haut ; changer un curseur prend effet immédiatement, sans
redémarrage.

| Groupe | Réglages |
|---|---|
| Général | Module actif — **interrupteur maître** : jeu fermé, plus rien ne se joue, même si les cases ci-dessous restent cochées. Consulter son profil, son inventaire et le classement reste possible. |
| Bagarres | Mise min/max · écart de niveau toléré · nombre de manches |
| Fouille | Autorisée · chance de réussite · chance Piégeur · part prise · part perdue en cas d'échec · délai · solde minimum d'une cible |
| Contrats | Autorisés · minimum · maximum |
| Paris | Autorisés · minimum · gain d'un pari gagnant |
| Coffre à coussins | Garantie disponible · prix · durée · prix des objets |
| Progression | Niveau maximum · XP du vainqueur · XP du perdant · points gagnés par niveau |

Les curseurs affichés sont **exactement** ceux qu'un service lit réellement. Un réglage sans
effet est pire que son absence : il fait croire au problème résolu.

Un seul chiffre reste volontairement en dur : **les 5 % d'arnaque de la garantie**. C'est la
blague de la mécanique — à 0 l'achat n'a plus d'histoire, à 100 ce n'est plus une garantie.

---

## Pour les développeurs

| Où | Quoi |
|---|---|
| `nexus-core/src/domain/entities/coussin.rs` | Règles pures : classes, dégâts, résolution, niveaux. Aucune I/O, testable sans base. |
| `nexus-core/src/domain/entities/coussin_shop.rs` | Catalogue des objets. Source unique des libellés. |
| `nexus-core/src/application/coussin_*_service.rs` | Cas d'usage : bagarre, fouille, contrats, paris, garantie, inventaire. |
| `nexus-api/src/adapters/inbound/http/handlers/coussin.rs` | Routes `/api/coussin/...` |
| `nexus-bot/src/main.rs` · `embeds.rs` | Commandes slash et rendu Discord. |
| `nexus-api/migrations/021_coussin_piege.sql` | Renommage depuis « Coup de Coude ». |
| `web/src/components/pages/GamesPage.vue` · `NexusCoussinPage.vue` | Page joueur et back-office. |

Le hasard (dés, réussite d'une fouille, arnaque d'une garantie) est **injecté par les cas
d'usage**, jamais tiré dans le domaine : c'est ce qui permet de rejouer une bagarre à
l'identique en test.

Les **clés techniques** des objets (`rage`, `mindgame`, `coup_traitre`…) datent d'avant le
changement de nom. Elles sont écrites dans les inventaires existants : seul l'affichage a
changé, les renommer viderait le sac de tout le monde.
