# TODO — Super admin via .env + page Comptes

## Objectif

Permettre à un (ou plusieurs) compte Discord identifié par son **User ID** dans le `.env` d'avoir accès à **toutes les pages** du site sans restriction, et créer une page **Comptes** pour gérer les accès des autres utilisateurs page par page.

---

## 1. Super admin via `.env`

### Variable d'environnement
```
SUPER_ADMIN_DISCORD_IDS=123456789012345678,987654321098765432
```
- Liste d'IDs Discord (snowflakes) séparés par virgule, pour en gérer plusieurs.

### Côté API (`services/api`, Rust)
- Charger la variable au démarrage (config).
- Middleware d'auth : après avoir résolu l'utilisateur courant via OAuth Discord, comparer son `discord_id` à la liste.
- Si match → flag `is_super_admin = true` injecté dans le contexte de la requête.
- **Bypass** de tous les checks RBAC / permissions (court-circuit dans le guard).
- Endpoint à exposer : `GET /api/me` doit renvoyer `is_super_admin` pour que le front puisse adapter l'UI.

### Côté web (`apps/web`)
- `authStore` : stocker `isSuperAdmin` à la connexion.
- Router : tous les `meta.requiresPermission` sont ignorés si `isSuperAdmin === true`.
- `SidebarNav` : afficher **toutes** les entrées de menu si super admin.
- Aucun filtrage côté composant pour un super admin.

---

## 2. Page "Comptes" (gestion des accès par utilisateur)

### Fonctionnalités
- Lister tous les utilisateurs qui se sont **connectés au site** (login OAuth Discord).
- Pour chaque utilisateur, afficher :
  - Avatar, pseudo, Discord ID
  - Date de première connexion / dernière connexion
  - Liste des pages auxquelles il a accès (cases à cocher)
- Action : modifier les accès (cocher/décocher par page) → sauvegarde via API.
- Filtre / recherche par pseudo ou ID.
- **Accessible uniquement aux super admins** (au minimum), éventuellement déléguable à un rôle "gestion comptes".

### Côté API
- Table DB (à créer ou étendre) :
  - `site_accounts` : `id`, `discord_id`, `username`, `avatar_url`, `first_login_at`, `last_login_at`
  - `site_account_permissions` : `account_id`, `page_key`, `can_access` (bool)
  - (option granularité : `can_read`, `can_write`)
- Endpoints :
  - `GET /api/accounts` — liste paginée
  - `GET /api/accounts/:id` — détail + permissions
  - `PUT /api/accounts/:id/permissions` — body `{ page_key, can_access }[]`
- Lors du login OAuth : upsert dans `site_accounts` (créer si nouveau, mettre à jour `last_login_at`).
- Liste des `page_key` : centralisée dans une constante côté API + côté web (source de vérité partagée, idéalement dans `services/proto` ou un fichier généré).

### Côté web
- Nouvelle page `apps/web/src/components/pages/AccountsPage.vue`.
- Route `/accounts` avec guard super admin.
- Entrée dans `SidebarNav` (visible super admin uniquement).
- Composants à réutiliser : `DataTable`, `RuleCard` (pattern existant pour les permissions).
- Store Pinia `accountsStore` (CRUD + cache).

---

## 3. Questions à trancher avant de coder

1. **Page `RbacPage.vue` existe déjà** — vérifier si c'est :
   - rôles Discord du serveur (et donc indépendant), OU
   - rôles applicatifs du site (et donc à étendre / fusionner avec la nouvelle page Comptes).
2. **Granularité** : juste accès à la page (boolean), ou lecture/écriture séparées ?
3. **DB existante** : y a-t-il déjà une table de comptes site, ou faut-il la créer ?
4. **Liste des pages** : générer dynamiquement depuis le router web, ou maintenir une liste manuelle côté API (plus sûr, plus stable) ?
5. **Audit** : logger les changements de permissions (qui a modifié quoi, quand) ?

---

## 4. Ordre d'implémentation suggéré

1. Variable `SUPER_ADMIN_DISCORD_IDS` + bypass API + flag `isSuperAdmin` dans `/me`.
2. Adapter `authStore` + router + `SidebarNav` côté web.
3. Migration DB `site_accounts` + `site_account_permissions`.
4. Upsert au login OAuth.
5. Endpoints CRUD comptes + permissions.
6. Page `AccountsPage.vue` + store + route.
7. Audit log (optionnel, phase 2).
