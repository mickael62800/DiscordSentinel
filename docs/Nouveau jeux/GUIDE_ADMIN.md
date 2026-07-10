# 🛠️ Influence — Guide d'installation & d'administration

> Documentation technique pour installer, configurer et exploiter le jeu **Influence**
> sur un serveur Discord. Public : administrateurs de serveur et intégrateurs du bot.
> Pour l'architecture interne du code, voir [`ARCHITECTURE.md`](./ARCHITECTURE.md).

---

## 1. Prérequis

- Le bot **DiscordSentinel** présent sur le serveur, composant **`influence-bot`** activé.
- Permissions Discord accordées au bot :
  - **Gérer les rôles** et **Gérer les salons** (création dynamique de QG d'organisations + rôles),
  - **Envoyer des messages / Intégrer des liens** dans les salons du jeu,
  - Le **rôle du bot placé haut** dans la hiérarchie (pour gérer les rôles d'org).
- Base de données à jour (migrations `influence_*`, à partir de `329_influence_game.sql`).

---

## 2. Activation du composant

Le jeu se pilote depuis la page **Composants** de l'interface web (système
schéma-driven : les réglages apparaissent automatiquement).

1. Interface web → **Composants** → **`influence-bot`**.
2. Activer le composant (`enabled`).
3. Régler les paramètres (voir §4).

---

## 3. Structure de salons recommandée

À créer manuellement, ou en 1 clic via la feature **Guild Backup / Restore**
(un snapshot JSON reproduit toute l'arborescence — voir §6).

### Version complète

| Catégorie | Salons | Type |
|---|---|---|
| **🏛️ Place publique** | `#agora`, `#annonces-du-monde`*, `#place-du-marché`, `#présentation-citoyens` | Texte |
| **👤 Citoyen** | `#mon-profil`, `#transferts`, `#classements`* | Texte |
| **🏢 Organisations** | `#registre-des-orgs`*, `#création-organisation`, `#relations-inter-orgs`* | Texte |
| **⚖️ Politique** | `#parlement`, `#votes-en-cours`, `#journal-officiel`*, `#élections`, `#tribune-campagne` | Texte + Vocal |
| **📰 Médias & info** | `#la-presse`*, `#rumeurs`, `#petites-annonces-info`, `#salle-de-rédaction` | Texte + Vocal |
| **📜 Mémoire** | `#archives`*, `#fil-d-actualité`* | Texte |
| **🔒 Coulisses** *(option)* | `#complots`, `#tribunal` | Texte restreint |

`*` = salon en **lecture seule** conseillée (le bot y poste, les joueurs lisent).

### Version minimale (démarrage)
`#agora` · `#mon-profil` · `#registre-des-orgs` · `#votes-en-cours` · `#annonces-du-monde`

---

## 4. Paramètres configurables (`config_schema`)

Réglables par serveur depuis la page **Composants** (chargés côté bot via
`guild_settings.rs`). Exemples de clés :

| Clé | Défaut | Description |
|---|---|---|
| `influence_mandate_days` | `14` | Durée d'un mandat politique (jours) |
| `influence_org_creation_cost` | `1000` | Coût en argent pour fonder une organisation |
| `influence_org_role_cost` | `2000` | Coût du rôle Discord d'une organisation (gratuit pour un modérateur) |
| `influence_org_category_id` | *(vide)* | Catégorie Discord où ranger les **salons privés d'organisations** (vide = le bot crée/trouve « 🏢 Organisations ») |
| `influence_law_debate_hours` | `48` | Durée du débat d'une loi avant vote (h) |

> **Salon privé automatique par organisation** : à chaque `/org create`, le bot
> crée un **salon texte privé** (sous la catégorie ci-dessus), visible des seuls
> membres. Le fondateur y a accès immédiatement ; chaque `/org join` ajoute
> automatiquement l'accès au nouveau membre. Prérequis bot : **Gérer les salons**.

> Ajouter un paramètre = **une migration** qui étend `config_schema` sur
> `bot_definitions` (motif `UPDATE … WHERE NOT (@>)`). **Aucun code web** à
> écrire : le formulaire se génère tout seul.

---

## 5. Référence des commandes (état actuel)

| Commande | Sous-commandes | Rôle | Statut |
|---|---|---|---|
| `/influence-profil` | `[citoyen]` | Profil (chiffres si soi, paliers sinon) | ✅ Live |
| `/capital` | — | Capitaux exacts + historique du joueur | ✅ Live |
| `/transfert` | — | Conversion d'un capital en un autre | ✅ Live |
| `/org` | `create`, `info`, `join`, `membres` | Gestion des organisations | ✅ Live |
| `/vote` | — | Ouvrir un vote interne à une organisation | ✅ Live |
| `/loi` | `propose` | Proposer une loi au vote | ✅ Live |
| `/enquete` | — | Enquête payante sur un citoyen (résultat différé) | ✅ Live |
| `/dossier` | — | Consulter ses informations secrètes (intel) | ✅ Live |
| `/reveler` | — | Révéler une info → scandale public | ✅ Live |
| `/actu` | — | Fil d'actualité du serveur | ✅ Live |
| `/archives` | — | Mémoire du serveur (grands événements) | ✅ Live |

> Commandes de phases ultérieures (élections/candidatures/mandats, alliances
> inter-orgs, motions de défiance) : voir la feuille de route dans
> [`ARCHITECTURE.md`](./ARCHITECTURE.md) §4-5.

---

## 6. Déploiement rapide des salons via Guild Backup

La structure §3 peut être capturée une fois puis **restaurée en 1 clic** sur tout
nouveau serveur :

1. Créer la structure sur un serveur modèle.
2. Interface web → **Guild Backup** → **Capturer** (snapshot JSON versionné).
3. Sur le serveur cible → **Restaurer** (option **wipe** possible pour repartir propre).

Le remapping des IDs (rôles, catégories, salons, overwrites) est automatique.
Prérequis bot : *Gérer rôles/salons* + rôle haut dans la hiérarchie.

---

## 7. Le monde vivant (worker)

Le jeu **continue d'évoluer hors ligne** via `sentinel-worker` (domaine
`influence`) : expiration des mandats, ouverture/dépouillement des scrutins,
passage des lois en vote, progression des enquêtes, événements mondiaux
occasionnels. Rien à configurer côté salon — le bot poste les résultats dans les
salons d'annonces / journal officiel / archives.

---

## 8. Points d'attention exploitation

1. **Rôles d'organisation** : le bot crée des rôles/salons par org → surveiller le
   plafond Discord (250 rôles / 250 salons) sur les serveurs très actifs.
2. **Archives = append-only** : le salon `#archives` reflète une table jamais
   purgée. Ne pas s'attendre à pouvoir « nettoyer » l'historique.
3. **Salons en lecture seule** : retirer *Envoyer des messages* aux joueurs sur
   les salons marqués `*` — le bot garde le droit d'y poster.
4. **Multi-serveur** : toutes les données portent `guild_id` ; deux serveurs sont
   totalement cloisonnés et évoluent indépendamment.

---

## 9. Pour aller plus loin

- **Game Design Document** : `docs/Nouveau jeux/01.md` → `07.md` (vision, capitaux,
  organisations, politique, information).
- **Architecture technique** : `docs/Nouveau jeux/ARCHITECTURE.md` (couches
  hexagonales, modèle de données, phases d'implémentation).
- **Guide joueur** : `docs/Nouveau jeux/GUIDE_JOUEUR.md`.
</content>
</invoke>
