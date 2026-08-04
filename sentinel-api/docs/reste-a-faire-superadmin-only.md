# Reste a faire — passage du back-office en superadmin-only

**Commit de reference :** `f07d7406` (main)
**Date :** 2026-08-04

Le refactor est fonctionnel et compile (`cargo check --workspace` : 0 erreur,
`vue-tsc` : 0 erreur hors dependance manquante, cf. §5). Ce document liste ce
qui reste, par ordre de criticite.

---

## 1. BLOQUANT AU DEPLOIEMENT — ordre des operations

Deux actions, **dans cet ordre imperatif** :

1. **Renseigner `SUPERADMIN_USER_IDS`** dans le `.env` de production
   (format : `SUPERADMIN_USER_IDS=123456789012345678,234567890123456789`).
2. **Appliquer la migration** `sentinel-api/migrations/007_drop_rbac_multi_roles.sql`.

Le gate est **fail-closed** : liste vide = aucun utilisateur web n'entre. Comme
la migration supprime `api_user_guilds`, se tromper d'ordre laisse un
back-office inaccessible **sans table ou reparer le tir a la main**. La seule
issue serait alors la variable d'environnement + redemarrage.

`.env.example` ne documente pas encore `SUPERADMIN_USER_IDS` : a ajouter, avec
un commentaire sur le comportement fail-closed.

## 2. BLOQUANT CI — la suite de tests ne compile plus

**Elle n'a jamais ete compilee pendant le refactor** (`cargo check` a ete lance
sans `--all-targets`, sur demande). Les references aux symboles supprimes
subsistent et casseront `cargo test` :

- `sentinel-api/tests/test_helpers.rs` — stubs `StubRbac`,
  `ManageComponentMinRoleUseCase`, `ManageComponentVisibilityUseCase`, et les
  entites `component_min_role` / `component_visibility` (~lignes 2344-2600).
- `sentinel-api/src/tests/config.rs` — construit un `Config` avec
  `rbac_global_gate` et `rbac_global_gate_audit`, champs supprimes.
- `sentinel-api/tests/integration/rbac_http/http.rs` — teste des endpoints qui
  n'existent plus ; le dossier est a supprimer entierement.
- Une quinzaine d'autres fichiers sous `sentinel-api/tests/integration/`
  passent un `RoleContext` ou un `Role::` aux handlers dont la signature a
  change (`bot_config`, `notes`, `purge_http`, `security`, `tickets`, `voice`,
  `watched_users`, `members_http`, `exports_http`, `discord_roles_http`,
  `moderation_extra`, `bot_persistence_http`, `misc/remaining`).

`sentinel-core/src/domain/enums/system/tests/role.rs` teste l'enum `Role`, qui
existe toujours (il sert encore aux tickets et a l'automod) — a conserver.

Marche a suivre : `cargo check --workspace --all-targets`, puis corriger le
listing. L'essentiel est mecanique (retrait d'un argument), sauf `rbac_http` et
les stubs de `test_helpers` qui sont a supprimer.

## 3. Nettoyage cosmetique du front (sans effet fonctionnel)

`myRoleStore` et `componentVisibilityStore` ont ete vides de leur logique et de
leurs appels reseau : `visible(...)` renvoie toujours `true`, `isSuper` toujours
`true`, `role` toujours `owner`. Les composants qui les interrogent compilent et
se comportent correctement, mais leurs conditions sont desormais constantes.

Environ vingt fichiers concernes, dont `DockerAdminSection.vue`,
`ModerationBansTab.vue`, `ModerationJournalTab.vue`, `ConfessionsTable.vue`,
`SecurityEventsList.vue`, `VoiceChannelsHistoryList.vue`, `MemberHomePage.vue`,
`useDashboardSections.ts`, `useUniverse.ts`, `useAppInit.ts`.

`useDashboardSections.ts` expose encore `rbacKeyForPath`, qui n'a plus d'appelant
depuis le retrait du guard de route dans `main.ts`.

Retirer tout ca supprimerait les deux stores et leurs composables. Volontairement
non fait dans le commit du refactor : churn important sur des fichiers d'affichage
deja touches, pour un gain nul a l'execution.

## 4. Warnings de compilation

`cargo check -p sentinel-api` remonte ~196 warnings, essentiellement des
parametres `user: Option<Extension<WebUser>>` reinjectes dans des handlers qui
ne s'en servent pas. Deux options : les prefixer `_user`, ou les retirer quand
le handler n'a pas besoin d'identite. A faire en meme temps que §2, le
compilateur listant les deux.

## 5. Dependance web manquante

`vue-tsc` signale `@vueuse/motion` introuvable. Il est declare dans
`web/package.json` (`^3.0.3`) mais absent de `node_modules` : c'est un
`npm install` qui manque, anterieur a ce refactor et sans rapport avec lui.

## 6. Configuration nginx

`/api/auth/nexus-access` ne lit plus l'en-tete `X-Guild-Id` (la decision ne
depend plus d'une guilde). Si la directive `auth_request` de la passerelle
`/nexus-api/` le transmet encore, c'est devenu inutile — a nettoyer au passage,
sans urgence.

## 7. Dette annexe reperee, non traitee

Deux points releves pendant l'audit des bots, laisses en l'etat car ils
relevent d'une decision produit :

- `sentinel-bot/src/modules/moderation/commands/mute.rs` — une duree absente est
  etiquetee « permanent » et journalisee `mute_permanent`, mais le timeout
  Discord applique vaut `default_mute_duration_secs` (1 h par defaut). Discord
  ne sait pas faire de timeout permanent : le libelle ment a l'utilisateur. A
  trancher entre renommer le libelle ou basculer sur un role muet.
- `sentinel-api/.../handlers/moderation/commands/expirations.rs` — seul handler
  de moderation sans `has_mod_permission` explicite. Couvert par
  `default_member_permissions` cote Discord, mais depareille avec le reste du
  module.

## 8. Branche

`refactor/back-office-superadmin-only` a ete fusionnee dans `main` en
fast-forward. Elle peut etre supprimee sur le remote.

## 9. Avertissement sur le contenu du commit

Le commit `f07d7406` embarque aussi le chantier « server builder »
(`guild_structure`, `ServerBuilderPage`, `useServerBuilder`, `channel_access`,
`channel_plan`, `ChannelAccessEditor`, `guildStructureService`), qui etait en
cours et non commite dans l'arbre de travail. Ses modifications partageaient les
memes fichiers que le refactor (`router.rs`, `routes/mod.rs`,
`entities/system/mod.rs`, `web/router/index.ts`) et n'etaient pas separables
sans casser la compilation. Ce code n'a pas ete relu ni teste dans le cadre de
ce chantier.
