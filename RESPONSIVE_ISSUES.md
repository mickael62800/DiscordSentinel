# Responsive & Architecture — État au 2026-05-05

> Suivi consolidé des soucis d'affichage mobile/tablette détectés sur l'app web,
> et du travail d'architecture associé (AdminPageShell, normalisation CSS).
> Cible viewport : smartphone portrait (375-414px).

> **Convention** :
> - 🔴 **Bloquant** : élément déborde, illisible, casse la lecture.
> - 🟠 **Gênant** : visible mais inconfortable.
> - ⚪ **À vérifier** : signalé, à reproduire en browser.
> - ✅ **Fixé** : commit lié indiqué.

---

## ✅ Architecture — refactor mai 2026

### `AdminPageShell` (commits `ede4821e` → `a8a3cb45`)

Composant `web/src/components/layouts/AdminPageShell.vue` factorise le boilerplate header/lede/actions partagé par toutes les pages admin.

**API** :
- props : `title` (string), `icon` (emoji optionnel), `width` (`constrained` | `wide` | `narrow`)
- slots : `lede`, `actions` (côté droit du header), default

**Migré** (26 pages) : TempRoles, Sponsorships, Blackjack, Automod, NameHistory, Reminders, Strikes, SystemOps, Wheel, Slot, Welcome, LevelsConfig, CoudeSocial, VoiceThemes, Review, Tournament, TauntsConfig, Security, VoiceChannels, Coude, Rbac, RolePanelEdit, ComponentConfig, Wallet, ServerSecurity, Games.

**Non migré** :
- `EvidencePage`, `NotesPage` — header conditionnel `v-if="!props.embedded"` (page utilisable en standalone OU embarquée dans un drawer)
- `AiDatasetPage` — bloque le mobile via `.dataset-page > :not(.mobile-only-block) { display: none }`, structure spécifique
- Pages avec headers custom : `MembersPage`, `ModerationHubPage`, `AuditPage`, `DiscordRolesPage`, `LevelsPage` (chacune a sa logique : count badge, cross-link, animation shimmer, etc.)

**Gain** : ~470 lignes de boilerplate supprimées, responsive 768px du header inclus pour toutes les pages migrées.

### Renommage CSS partagé (commit `a3b76069`)

`_moderation-advanced-shared.css` → `_admin-page-shared.css` (26 imports mis à jour). Le fichier est un boilerplate générique de page admin (cards, lookup, table, btn-*), pas spécifique à la modération.

### Normalisation des inputs (commit `fa1eb10a`)

Standard imposé : `.field-input` (`ConfigFieldRow.vue`) =
- padding **8px 12px**
- border-radius **6px**
- font-size **13px**
- background `var(--bg-card)`
- border `1px solid var(--border)`

Classe globale équivalente : `.app-input-base` dans `global.css` — à appliquer sur tout futur `<input>`/`<select>`/`<textarea>` non spécialisé.

---

## ✅ Fixés au cours de l'audit 2026-05-05

| Item | Fichier | Commit |
|---|---|---|
| Inputs hétérogènes (3 outliers) | `CancelConductBanModal`, `ModerationBansTab`, `SecurityEventsList` | `fa1eb10a` |
| Onglets qui débordent (impact global) | `AppTabs.vue` (scroll horizontal mobile <640px) | `001aab4f` |
| MembersPage panneau latéral 720px | stack vertical sous 900px | `001aab4f` |
| LevelsPage KPI sous 480px | grid `auto-fit minmax(120px,1fr)` | `001aab4f` |
| VoiceChannels Active/History tables | `display: block; overflow-x: auto` sous 768px | `001aab4f` |
| AuditEventDetail `.mono` snowflakes | `overflow-wrap: anywhere; word-break: break-all` | `485df934` |
| LevelsLeaderboardTab user-row | flex-wrap + actions stackées sous 600px | (pending commit) |
| PaginationBar | flex-wrap + boutons compacts sous 640px | (pending commit) |

---

## ✅ Déjà responsive (vérifié au code)

Pages avec breakpoints `@media` déjà en place :

- **AutomodPage**, **AutomodDetectionsTimeline** : header timeline déjà stacké sous 640px ✓
- **SlotPage**, **WheelPage**, **LevelsConfigPage** : grids stackées
- **WalletPage** : `.danger-zone` stackée à 768px, KPI + padding compact à 480px
- **RbacPage** :
  - `RbacUsersTable` → `overflow-x: auto` + breakpoint 480px
  - `ComponentVisibilityGrid` → convertit table en cards verticales sous 768px
- **MembersPage** : `.filters` en colonne, `.member-name`/`.member-id` ellipsis, sort/search 100% à 480px
- **SecurityPage** : stats-grid auto-fit, search-input clamp
- **TempRolesPage**, **SponsorshipsPage** : formulaires en `grid-template-columns: 1fr`
- **BlackjackPage**, **CoudePage** : hero stacké via `AdminPageShell` (auto-responsive)
- **DiscordRolesPage** : header/toolbar stacké à 768px
- **VoiceChannelsPage** : stats-row auto-fit à 768px
- **AuditEntriesList** : `min-width: 0` + `overflow-wrap: anywhere`
- **ComponentSelectorSection** : grid 5 cols → 1 col sous 640px ✓
- **ServerSecurityPage** tables (Attacks/Network/Integrity tabs) : utilisent `.data-table` global, hérite du responsive `display: block; overflow-x: auto` à 768px ✓
- **AiDatasetPage** : page entière masquée sous 768px ✓ (déjà fait, doc précédente obsolète)

Inputs alignés sur le standard `.field-input` : `AppInput`, `RoleSelect`, `ChannelSelect`, `EnumSelect`, `NumberInputWithUnit`, `ConfigFieldRow`, modals (AddWatch, DiscordRoleCreate/Edit, CancelConductBan), filtres (AuditFilters, MembersPage, ModerationBansTab, SecurityEventsList), inputs spécialisés (LevelsRewardsTab, MemberDetailDrawer, LevelsPage `xp-mode-select`).

---

## 🔴 Reste à vérifier en browser sur 375px

Items du doc précédent **non confirmés en code** — nécessitent une reproduction visuelle avant de coder un fix.

### ModerationHubPage
- ⚪ KPI label "Infractions" déborde (probablement `UserDossierPanel.summary-card` ou `MemberDetailDrawer` — déjà flex-wrap, pas trouvé d'overflow réel).

### ServerSecurityPage onglets
- Demande user antérieure : remplacer la barre d'onglets `AppTabs` par un `<select>` en mobile pour les pages avec >5 onglets (ServerSecurity en a 5+). Pas implémenté.
- Decision : `AppTabs` a maintenant scroll horizontal global (commit `001aab4f`) — peut-être suffisant. À valider en browser.

### Pages potentiellement problématiques (non testées)
- **AnnouncementsPage** : modale et chips multipicker (refait récemment).
- **ConfessionsPage** : moderation page complète.
- **TopBar** : si plusieurs guilds, dropdown + avatar + logo serrés.
- **ServerHealthPage** : graphiques Chart.js qui peuvent overflow.

---

## 📌 Méthodologie

1. **Tester sur 375px** (Chrome DevTools iPhone SE) chaque batch de fixes responsive avant d'enchaîner.
2. **Patterns réutilisables** :
   - Tables denses → `.data-table` (global responsive automatique sous 768px) ou wrapper `overflow-x: auto`
   - KPI grids → `grid-template-columns: repeat(auto-fit, minmax(120px, 1fr))` sous 480px
   - Headers titre+actions → utiliser `AdminPageShell` (auto-responsive 768px)
   - Snowflakes/hashes → `overflow-wrap: anywhere; word-break: break-all`
   - Onglets → géré globalement par `AppTabs` (scroll horizontal mobile)
   - Pagination → géré globalement par `PaginationBar` (wrap mobile)
3. **Ne pas se fier au doc seul** — vérifier le code avant de coder un fix.
