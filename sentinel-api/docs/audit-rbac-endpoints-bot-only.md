# Acces au back-office — modele actuel

**Mise a jour :** 2026-08-04
**Statut :** le RBAC multi-roles a ete supprime ; ce document decrit ce qui le remplace.

---

## Le modele

Un seul mode d'acces humain : les Discord user IDs listes dans
`SUPERADMIN_USER_IDS` (.env). Tout le reste est refuse.

`superadmin_middleware` (`adapters/inbound/http/middleware/superadmin.rs`)
applique la regle unique, dans cet ordre :

1. **Dev mode** (`API_KEY` vide) → pass-through, pour ne pas casser le local.
2. **`AuthKind::Internal`** (bot et workers, `Authorization: Bearer <API_KEY>`)
   → pass-through.
3. **Utilisateur web** : son `X-Discord-Token` est resolu en identite Discord
   (cache Redis + `GET /users/@me`), qui doit figurer dans
   `SUPERADMIN_USER_IDS` → sinon **403**.

L'identite resolue est injectee en extension `WebUser` : les handlers qui
attribuent une action a son auteur (audit, `deleted_by`, acteur d'un reset) la
lisent via `Option<Extension<WebUser>>`, `None` signifiant « appel interne ».

**Fail-closed :** si `SUPERADMIN_USER_IDS` est vide, aucun utilisateur web ne
passe. C'est volontaire — mieux vaut un back-office inaccessible qu'ouvert. Les
services internes continuent de fonctionner via l'`API_KEY`.

## Ce qui a disparu, et pourquoi ca reglait le probleme precedent

Ce fichier documentait auparavant une dette : `require_internal` etait ecrit,
teste, mais jamais appele, et toute la surface de **lecture** bot-only
echappait au `global_rbac_gate` (qui ne gate que les methodes mutantes). Un
`GET /api/age-bans/due` non protege exposait ainsi, a tout utilisateur web
whitelist, les bans en attente de toutes les guilds.

Le passage en superadmin-only supprime la classe de probleme plutot que de la
traiter cas par cas : il n'y a plus d'utilisateur web « intermediaire » a qui
une lecture pourrait fuiter. Ont ete supprimes :

- les middlewares `rbac`, `whitelist`, `guild_auth`, `global_rbac`,
  `component_gates` ;
- les roles applicatifs (`owner` / `admin` / `moderator` / `viewer`), les
  invitations a usage unique, les gates de visibilite et de `min_role` par
  composant — code, use cases, repos et tables (migration
  `007_drop_rbac_multi_roles.sql`) ;
- cote web : `rbacService`, `invitationsService`, la page RBAC, le gestionnaire
  d'invitations et les grilles de visibilite.

`require_internal` a disparu avec le reste : la distinction « appelant web vs
service interne » est desormais portee par le middleware unique, en un seul
endroit.

## Points d'attention pour la suite

- **`SUPERADMIN_USER_IDS` est le seul point de defaillance de l'acces.** Le
  renseigner avant de deployer ; le perdre verrouille le back-office (la
  reparation passe par la variable d'environnement, plus par du SQL).
- **`single_guild` reste actif** : il refuse les requetes visant une autre
  guilde que celle configuree. Il ne depend plus de `guild_auth` (son
  extraction de `{guild_id}` a ete inlinee).
- **Les conditions d'interface subsistent cote web** (`visible(...)`,
  `isSuper`) mais repondent desormais toujours vrai ; les stores ont ete vides
  de leur logique et de leurs appels reseau. Les retirer des ~20 composants qui
  les appellent est un nettoyage cosmetique restant, sans effet fonctionnel.
