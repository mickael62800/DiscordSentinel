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

~~`.env.example` ne documente pas encore `SUPERADMIN_USER_IDS`~~ **FAIT** :
variable ajoutee dans la section « API Backend » avec un commentaire sur le
comportement fail-closed. Reste l'action de deploiement elle-meme (renseigner la
valeur en prod + migration), non automatisable ici.

## 2. ~~BLOQUANT CI — la suite de tests ne compile plus~~ FAIT

`cargo check --workspace --all-targets` : 0 erreur, 0 warning. Detail des
corrections :

- `tests/integration/rbac_http/` supprime (dossier + entree `[[test]]` du
  `Cargo.toml`).
- `src/tests/config.rs` et `tests/test_helpers.rs` : champs `rbac_global_gate`
  / `rbac_global_gate_audit` retires des constructeurs `AppConfig` / `AppState`.
- `test_helpers.rs` : stubs `StubInvitations`, `StubComponentMinRole`,
  `StubComponentVisibility`, `StubRbac` supprimes, ainsi que leurs champs dans
  le builder de state et le helper `request_with_rbac`.
- Les tests `*_with_rbac_*` (assertions 403 par role + seed `api_user_guilds`)
  ont ete supprimes : ils validaient un sous-systeme qui n'existe plus. Les
  actions sous-jacentes restent couvertes par les tests non-rbac de chaque
  module. Helpers devenus morts (`seed_rbac*`, `send_request`, `pool`,
  `insert_note`, `build_state_full_mocks`, imports `Uuid`) retires au passage.

<details><summary>Detail d'origine (conserve pour reference)</summary>

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

</details>

## 3. ~~Nettoyage cosmetique du front~~ FAIT

Les deux stores (`myRoleStore`, `componentVisibilityStore`) et leurs composables
(`useMyRole`, `useComponentVisibility`) ont ete **supprimes**. `vue-tsc --noEmit`
: 0 erreur ; `eslint` : 0 erreur (les warnings `no-multi-spaces` restants sont
pre-existants, hors de ce chantier). Detail :

- Conditions constantes remplacees par leur valeur : `canManage` / `canBuild` /
  `canDelete` / `isOwner` / `hasAdminAccess` = `true` dans `ConfessionsTable`,
  `SecurityAttacksTab`, `SecurityNetworkTab`, `GuildBackupPage`,
  `ServerBuilderPage`, `ServerSecurityPage`, `MemberHomePage`.
- `v-if="visible('...')"` / `v-if="rbacVisible('...')"` retires des templates
  (`DockerAdminSection`, `ModerationBansTab`, `ModerationJournalTab`,
  `SecurityEventsList`, `VoiceChannelsHistoryList`) : les elements s'affichent
  toujours.
- `useDashboardSections.ts` : filtre `rbacVisible` retire, fonction morte
  `rbacKeyForPath` + table `PATH_RBAC_ALIASES` supprimees, tuile `config.rbac`
  (route `/rbac` inexistante) retiree.
- `useUniverse.ts` : `canAccessNexus` vaut desormais `true` (les deux univers
  sont toujours accessibles au superadmin).
- `useAppInit.ts` / `main.ts` : preloads `preloadMyRole` /
  `preloadComponentVisibility` retires.

**Signal superadmin cote client** (traite) : l'espace membre `/membre` est
public (visible sans connexion), donc `hasAdminAccess` toujours vrai affichait
le lien back-office meme a un visiteur anonyme. Ajout d'un flag `is_superadmin`
a l'identite Discord :

- API (`handlers/system/oauth.rs`) : le flag est calcule (appartenance a
  `SUPERADMIN_USER_IDS`) et propage dans le fragment du callback OAuth et dans
  la reponse de `POST /auth/refresh`.
- Web : `DiscordUser.is_superadmin` (config.ts), lu au callback
  (`AuthCallbackPage`) et au refresh (`http.ts`). `MemberHomePage` gate
  desormais le lien sur `user?.is_superadmin`. Filet de securite : `authStore`
  repose le flag quand `/api/auth/check-access` repond 200 (statut prouve),
  ce qui repare une identite en cache anterieure au flag.

Ce n'est qu'un confort d'affichage : l'autorisation reelle reste tranchee cote
serveur (403) par `superadmin_middleware` sur chaque route d'admin.

## 4. ~~Warnings de compilation~~ FAIT

Resorbes par le commit `14146a86`. `cargo check -p sentinel-api --all-targets`
ne remonte plus de warning.

## 5. ~~Dependance web manquante~~ FAIT

`@vueuse/motion` est desormais present dans `node_modules` (le `npm install`
manquant a ete fait).

## 6. ~~Configuration nginx~~ FAIT (commentaires)

`/api/auth/nexus-access` ne lit plus `X-Guild-Id`. Le SPA continue de l'envoyer
mais le handler l'ignore : aucun changement fonctionnel requis cote nginx. Les
commentaires de `web/nginx.conf` (bloc `/_nexus_auth`) ont ete corriges pour
decrire le modele superadmin-only au lieu du RBAC `nexus.access` par guild.

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
