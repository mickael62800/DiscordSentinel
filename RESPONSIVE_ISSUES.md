# Responsive — État au 2026-05-05

> Suivi des soucis d'affichage mobile/tablette détectés sur l'app web.
> Cible viewport : smartphone portrait (375-414px).

> **Convention** :
> - 🔴 **Bloquant** : élément déborde, illisible, casse la lecture.
> - 🟠 **Gênant** : visible mais inconfortable.
> - ⚪ **À vérifier** : signalé, à reproduire en browser.
> - ✅ **Fixé** : commit lié indiqué.

---

## ✅ Fixés au cours de l'audit 2026-05-05

| Item | Fichier | Commit |
|---|---|---|
| Inputs hétérogènes (3 outliers) | `CancelConductBanModal`, `ModerationBansTab`, `SecurityEventsList` + `.app-input-base` global | `fa1eb10a` |
| Onglets qui débordent (impact global) | `AppTabs.vue` (scroll horizontal mobile <640px) | `001aab4f` |
| MembersPage panneau latéral 720px | stack vertical sous 900px | `001aab4f` |
| LevelsPage KPI sous 480px | grid `auto-fit minmax(120px,1fr)` | `001aab4f` |
| VoiceChannels Active/History tables | `display: block; overflow-x: auto` sous 768px | `001aab4f` |
| AuditEventDetail `.mono` snowflakes | `overflow-wrap: anywhere; word-break: break-all` | `485df934` |

---

## ✅ Déjà responsive (vérifié au code, doc précédente obsolète)

Pages avec breakpoints `@media` déjà en place (768px ou 480px) :

- **AutomodPage**, **SlotPage**, **WheelPage**, **LevelsConfigPage** (kpi-row + grids stackés)
- **WalletPage** : padding/font + `.danger-zone` stackée à 768px
- **RbacPage** :
  - `RbacUsersTable` → `overflow-x: auto` + breakpoint 480px
  - `ComponentVisibilityGrid` → convertit table en cards verticales sous 768px
  - `ComponentMinRoleGrid` → idem (à confirmer en browser)
- **MembersPage** : `.filters` en colonne, `.member-name`/`.member-id` ellipsis, sort/search 100% à 480px
- **SecurityPage** : stats-grid auto-fit, search-input clamp
- **TempRolesPage**, **SponsorshipsPage** : formulaires en `grid-template-columns: 1fr`
- **BlackjackPage**, **CoudePage** : hero stacké, actions wrap à 768px
- **DiscordRolesPage** : header/toolbar stacké à 768px
- **VoiceChannelsPage** : stats-row auto-fit à 768px
- **AuditEntriesList** : `min-width: 0` + `overflow-wrap: anywhere` global sur entry-content

Inputs alignés sur le standard `.field-input` (8×12, r6, font 13) :
- `AppInput`, `RoleSelect`, `ChannelSelect`, `EnumSelect`, `NumberInputWithUnit` (atoms)
- `ConfigFieldRow.field-input` (référence)
- `AddWatchModal.modal-input`, `DiscordRoleCreate/EditModal.modal-input`
- `AuditFilters.search-input`/`event-select`
- `MembersPage.search-input`/`sort-select`
- `LevelsRewardsTab.converter-input`/`level-input`
- `MemberDetailDrawer.adjust-input`/`activity-date-input`
- `LevelsPage.xp-mode-select`

Standard global disponible : `.app-input-base` dans `global.css` (à utiliser sur les futurs inputs).

---

## 🔴 Reste à vérifier en browser sur 375px

Liste des items du doc précédent **non confirmés en code** — ces zones nécessitent une reproduction visuelle avant de coder un fix.

### ModerationHubPage
- ⚪ KPI label "Infractions" déborde de la card → vérifier si toujours présent après le fix `AppTabs` global.
- ⚪ Inputs filtres qui s'étendent → idem.

### ServerSecurityPage
> Page lourde, **non auditée page-par-page**. Beaucoup d'onglets / tables à vérifier individuellement :
- Top IPs, Échecs auth, SSH, Patterns, NGINX, Ban/protection, Réseau, Ports ouverts, Intégrité Docker, Fichiers critiques, Event serveur.
- Pattern attendu : `overflow-x: auto` sur wrapper de table, masquer colonnes secondaires sous 600px.
- Demande user : remplacer la barre d'onglets par `<select>` en mobile (à implémenter).

### AiDatasetPage
- 🟢 **Décision produit** : bloquer l'accès en mobile (`@media (max-width: 768px)` → message "Disponible uniquement sur desktop"). Non implémenté.

### ComponentConfigPage (Composants)
- ⚪ Liste des modules : 1 par ligne demandé sous 640px → vérifier le composant grid utilisé par la page.
- ⚪ Panneau de paramètres (clic module) : confirmer que `ConfigFieldRow` empile correctement.

### LevelsPage — onglets globaux/texte/vocal
- ⚪ Carte joueur (classement par niveau) — doc précédent dit "complètement explosée". À reproduire.
- ⚪ Convertisseur niveau ↔ XP — labels longs.
- ⚪ Header onglet "Rôle par niveau" — barre de contrôles.

### AutomodPage
- ⚪ Barre filtres timeline (titre + user_id + bouton sur 1 ligne) — à reproduire.

### SlotPage
- ⚪ Tableau "Spins récents" — à vérifier (peut hériter du fix `.data-table` global déjà en place).

### VoiceChannelsPage
- ⚪ Pagination — confirmer que `PaginationBar` ne déborde pas sur 375px.

### Pages probables à auditer (non vérifiées)
- **AnnouncementsPage**, **ConfessionsPage**, **AuditPage** (cards d'événements après fix `.mono`), **TopBar** (multi-guild), **ServerHealthPage** charts.

---

## 📌 Méthodologie pour la suite

1. **Tester sur 375px** (Chrome DevTools iPhone SE) chaque commit responsive avant d'enchaîner.
2. **Ne plus se fier au doc seul** — le code change vite, vérifier le code avant de coder un fix.
3. **Patterns réutilisables** :
   - Tables denses → wrapper `overflow-x: auto` + masquer colonnes secondaires sous 600px.
   - KPI grids → `grid-template-columns: repeat(auto-fit, minmax(120px, 1fr))` sous 480px.
   - Headers titre+actions → `flex-direction: column` sous 768px.
   - Snowflakes/hashes → `overflow-wrap: anywhere; word-break: break-all`.
   - Onglets → géré globalement par `AppTabs` (scroll horizontal mobile depuis `001aab4f`).

---

## 📐 Référence inputs

Standard imposé : `.field-input` (`ConfigFieldRow.vue`) =
- padding **8px 12px**
- border-radius **6px**
- font-size **13px**
- background `var(--bg-card)`
- border `1px solid var(--border)`

Classe globale équivalente : `.app-input-base` dans `global.css` — à appliquer sur tout futur `<input>`/`<select>`/`<textarea>` non spécialisé.
