# Responsive — Problèmes à corriger

> ⚠️ **État actuel : catastrophique sur certains points.** Cette doc liste de manière exhaustive les zones cassées en affichage mobile/téléphone, page par page, pour pouvoir attaquer les fixes méthodiquement plus tard.

Suivi des soucis d'affichage mobile/tablette détectés sur l'app web.
Cible viewport : smartphone portrait (375-414px).

> **Convention** :
> - 🔴 **Bloquant** : élément déborde, illisible, casse la lecture.
> - 🟠 **Gênant** : visible mais inconfortable.
> - ⚪ **À vérifier** : signalé, à reproduire.

---

## ModerationHubPage

### 🔴 Onglets qui débordent
**Onglets** `Journal`, `Bannis actifs`, `Suivi utilisateur`, `Workflow` (et autres) dépassent de l'écran horizontalement.
**Cause probable** : barre d'onglets en `display: flex` sans `flex-wrap` ni `overflow-x: auto`, chaque onglet a une largeur fixe.
**Pistes de fix** :
- Soit `overflow-x: auto` + scroll horizontal des onglets (motion 1D, classique mobile).
- Soit `flex-wrap: wrap` (les onglets descendent sur plusieurs lignes).
- Soit en mobile : bouton `⋯` qui ouvre un menu déroulant des onglets (navigation compacte).
- **Idéal mix** : si N onglets ≤ 3 → wrap ; si > 3 → scroll horizontal avec gradient indicateur.

### 🟠 Inputs qui s'étendent jusqu'au bout du formulaire
Les champs de recherche / filtres prennent toute la largeur, sans marge ni regroupement visuel cohérent.
**Cause probable** : `width: 100%` ou `flex: 1` sans `max-width` ni regroupement dans une `.filter-row` à `flex-wrap`.
**Pistes de fix** :
- Limiter chaque input à `max-width: 100%` mais avec `min-width: 0` pour éviter l'étirement.
- Padding interne du conteneur réduit en mobile (`padding: 8px` vs 16px).
- Stacker verticalement avec `gap: 8px` au lieu d'aligner horizontalement.

### 🔴 Texte "Infraction" qui déborde de la card "Infractions 0/1"
Le **cadre KPI** ne déborde pas, mais le **libellé** "Infractions" déborde de sa card sur mobile.
**Cause probable** : `font-size` fixe (1rem ou plus), pas de `text-overflow: ellipsis`, ou padding insuffisant.
**Pistes de fix** :
- Réduire `font-size` du label en mobile (ex: 11px) avec `letter-spacing` réduit.
- Ajouter `overflow: hidden` + `text-overflow: ellipsis` + `white-space: nowrap` sur le label de KPI.
- Ou autoriser le label sur 2 lignes : `white-space: normal` + line-height tight.

---

## MembersPage

> **Cassée à TOUS les viewports** (même 4K), pas juste mobile.

### 🔴 Trois inputs côte à côte (filtre + tri + recherche)
Les trois contrôles `Tous les membres` / `Trier par nom` / `Rechercher par nom ou ID` doivent être **stackés sur 3 lignes**, peu importe le viewport.
**Fix** : `.filters { flex-direction: column }` (et plus `flex: row` avec gap horizontal).

### 🔴 Texte du membre déborde la card
Le nom + détails sortent du cadre dès que le pseudo est long. Visible même en 4K.
**Cause** : `.member-identity` (flex row) sans `min-width: 0` sur les enfants → refus de shrink, le contenu pousse hors du parent.
**Fix** : `min-width: 0` sur `.member-identity` + `.member-names`, `overflow: hidden` + `text-overflow: ellipsis` + `white-space: nowrap` sur `.member-name` et `.member-id`.

---

## 🎨 Cohérence des inputs — divergences globales

Audit confirmé : les inputs **ne sont PAS homogènes** entre les pages. 16 classes CSS distinctes au lieu d'un seul style.

### Standard à imposer (référence)
`ConfigFieldRow.vue → .field-input` :
- padding : **8px 12px**
- border-radius : **6px**
- font-size : **13px**
- background : `var(--bg-card)`
- border : `1px solid var(--border)`

C'est aussi ce qu'utilisent `RoleSelect`, `ChannelSelect`, `EnumSelect`, `NumberInputWithUnit`. Ces 5 atoms sont déjà cohérents entre eux. ✓

### Outliers à corriger (par priorité)

| Priorité | Classe | Fichier | Divergence vs standard |
|---|---|---|---|
| 🔴 P1 | `.form-input` | ComponentConfigPage.vue:820 | padding 11×14, radius 8, font 14 (trop gros) |
| 🔴 P1 | `.form-input-number` | ComponentConfigPage.vue | padding 12×14, font 16 (trop gros) |
| 🔴 P1 | `.modal-input` | DiscordRolesPage.vue:496 | padding 11×14, radius 8, font 14 |
| 🟠 P2 | `.search-input` | AuditPage.vue:137 | radius 8 au lieu de 6 |
| 🟠 P2 | `.event-select` | AuditPage.vue:157 | radius 8 |
| 🟠 P2 | `.search-input` | MembersPage.vue:1207 | radius 8 |
| 🟠 P2 | `.sort-select` | MembersPage.vue:1224 | radius 8 |
| 🟠 P2 | `.adjust-input` | MembersPage.vue | padding 8×10, radius 8, font 14 |
| 🟠 P2 | `.modal-input` | AddWatchModal.vue | padding 10×md, radius md, font 14 |
| 🟡 P3 | `.activity-date-input` | MembersPage.vue:1900 | padding 4×8, font 12 (trop petit) |
| 🟡 P3 | `.xp-mode-select` | LevelsPage.vue:668 | padding 6×12 (compact) |
| 🟡 P3 | `.level-input` | LevelsPage.vue:962 | padding 6×8 (compact) |
| 🟡 P3 | `.converter-input` | LevelsPage.vue | font 16 |

### Stratégie de fix proposée

1. **Étape 1 — Variables CSS partagées** : créer une classe globale `.app-input-base` dans `global.css` avec les 5 valeurs standards. Toutes les classes locales l'héritent.
2. **Étape 2 — Migration outliers** : remplacer dans les 13 outliers les valeurs hardcodées par la classe globale (ou directement `padding/radius/font-size` aux valeurs standards).
3. **Étape 3 (optionnel)** : enrichir `AppInput.vue` (qui est aujourd'hui un wrapper minimaliste sans style) avec le style standard, pour que les futures pages aient un atom prêt à l'emploi.

**Estimé** : 7-9 fichiers à modifier, ~25 lignes de CSS, ~15 instances d'inputs. **1-2 commits**.

> **Décision user attendue** : on attaque cette homogénéisation **AVANT** ou **APRÈS** les fixes responsive ? Logique : avant, sinon on applique les fixes responsive sur des inputs qu'on va re-styler ensuite.

---

## ✅ Page Paramètres : RAS (validé user)

---

## ServerSecurityPage

### ✅ Vue d'ensemble : propre.
### ✅ Graphique "Trafic anormal" : OK.
### ✅ Onglet messages longs : OK.

### 🔴 Onglet "Top IPs par requêtes" — tableau tronqué
Colonnes visibles : `IP / Pays / Fournisseur` puis tout le reste (total requêtes, % bloqué...) coupé à droite.
**Fix** : `overflow-x: auto` ou masquer fournisseur d'accès en mobile.

### 🔴 Onglet "Échecs d'authentification" — colonnes tronquées
Affiche `When / Code / Méthode / root / root` mais tout est coupé à droite.
**Fix** : idem (overflow-x ou colonnes secondaires masquées).

### ⚪ Onglet "Tentatives SSH" / "Patterns suspicieux" / "NGINX"
User incertain de l'état → à vérifier au prochain audit.

### 🔴 Trop de tabulations imbriquées
Beaucoup d'onglets sur cette page (top IPs, auth fails, SSH, NGINX, ports, intégrité, etc.). En mobile, la barre d'onglets explose ou demande de scroller.
**Fix demandé par user** : remplacer la barre d'onglets par un **`<select>`** en mobile (1 choix dans une dropdown au lieu de N onglets visibles).
**Implémentation** : `@media (max-width: 700px)` cache les onglets en flex et affiche un `<select>` qui binde la même valeur active.

### 🔴 Onglet "Ban et protection" — discord_id tronqué
"Derniers logins Discord/host" : la colonne `discord_id` (snowflake 19 chiffres) déborde et est coupée.
**Fix** : `overflow-wrap: anywhere` ou format `…1234` (4 derniers chiffres) en mobile.

### 🔴 Onglet "Réseau" — géolocalisation tronquée
Géoloc IP coupée à droite. Fournisseurs d'accès OK ✓.
**Fix** : masquer pays ou ville en mobile, garder uniquement IP + fournisseur.

### 🔴 Onglet "Ports ouverts" — statut tronqué
La colonne `Statut` (ex: "port inattendu ouvert") déborde du cadre, le mot `ouvert` est coupé.
**Fix** : raccourcir le label en mobile (`inattendu` au lieu de `port inattendu ouvert`), ou `text-overflow: ellipsis`.

### 🔴 Onglet "Intégrité — Vulnérabilités Docker"
Colonnes `Image` / `CVE` débordent du cadre.
**Fix** : afficher CVE en pastille/chip avec scroll horizontal, ou mode card.

### 🔴 Onglet "Intégrité — Fichiers critiques"
Affiche `chemin / SHA-256 / statut`. SHA-256 (64 chars hex) explose totalement le cadre.
**Fix** :
- Tronquer le SHA en mobile (`abc123…789def`).
- Ou afficher uniquement le statut + chemin, le SHA accessible via tooltip ou modale.

### 🔴 Onglet "Event serveur"
Colonnes `Sévérité` / `Action` débordent pour certaines lignes.
**Fix** : `overflow-wrap: anywhere` sur le contenu, ou tronquer l'action longue avec ellipsis.

---

## AiDatasetPage — **catastrophe**

> **Décision user** : ne pas afficher cette page en mobile (export AI = uniquement desktop).

### 🟢 Action prioritaire : cacher en mobile
**Fix** : `@media (max-width: 768px)` sur la page entière → afficher un message "Disponible uniquement sur desktop" + masquer le contenu, OU bloquer la route avec un guard `viewport.isMobile`.

### 🔴 Si on garde le mobile : collecte des messages cassée
- Étiquettes `messages / channel / date` explosent.
- La **date** explose totalement.
- **Inputs incohérents avec le reste de l'app** : pas le même style que les autres pages.

### ⚪ Audit cohérence globale des inputs
User signale que les inputs de cette page sont **différents** des autres → suspicion de divergence stylistique généralisée.
**À faire** : grep des `<input>` / `<select>` dans `apps/web/src` et vérifier qu'ils utilisent les mêmes classes (`.field-input`, `.search-input`, `.role-select`...) ou un atom unique partagé. Pas de styles inline divergents.

---

## ServerHealthPage — section Docker (DockerAdminSection.vue)

### ✅ Vue d'ensemble : parfait, rien à toucher.
### ✅ Onglet Nettoyage : propre.

### 🔴 Onglet Conteneurs — tableau déborde
Colonnes visibles : `Nom / Image / État / [reste hors écran]`.
Les colonnes après `État` (ports, créé, actions ?) sont coupées tout à droite, invisibles en mobile.
**Fix** : `overflow-x: auto` sur le wrapper du tableau, ou masquer colonnes secondaires sous 600px (`Image`, `Créé`...). Les actions (▶ ⏹ ↻ 📋 🗑) doivent rester visibles ou accessibles via menu compact.

### 🔴 Onglet Images — tableau déborde
`Tag / ID` OK mais la colonne `Créé` (date de création) explose.
**Fix** : idem onglet Conteneurs, masquer colonne date ou format compact.

### 🔴 Onglet Volumes — tableau déborde
**Fix** : idem (overflow-x ou masquer colonnes accessoires).

### 🔴 Onglet Réseaux — tableau déborde
**Fix** : idem.

> Pattern commun : tous les tableaux Docker utilisent probablement la même classe CSS sans `@media` mobile. Un fix global dans `DockerAdminSection.vue` peut résoudre les 4 onglets en une fois (wrapper `overflow-x: auto` + media query qui masque colonnes secondaires `Créé`, `ID complet`, etc.).

---

## SystemOpsPage (Opérations modèles IA)

### 🔴 Tableau des modèles IA déborde
Colonnes visibles : `Modèle IA / Nom / Type / Statut / [colonnes coupées]`.
Le contenu sort du viewport mobile, on ne voit pas les dernières colonnes.
**Fix** :
- Wrapper du tableau en `overflow-x: auto` pour scroll horizontal.
- Masquer colonnes secondaires (`Type`, `Statut` ?) en `@media (max-width: 600px)` via `display: none`.
- Ou convertir chaque ligne en mini-card verticale en mobile (key/value).

---

## ComponentConfigPage (Composants) — **tout cassé**

### 🔴 Liste des modules — explosion à partir du 3ème
Affichage actuel : 1, 2, 3 modules sur la même ligne → débordement à partir du 3ème.
**Fix demandé par user** : 1 module par ligne en mobile (grille 1 colonne).
**Implémentation** : `grid-template-columns: 1fr` en `@media (max-width: 640px)` sur la grille des modules.

### 🔴 Panneau de paramètres (clic sur un module) — catastrophe
Tous les champs (salons, rôles, configs) sont sur la même ligne et explosent textes + cadres.
**Fix** :
- Chaque field row en pleine largeur, layout vertical (label en haut, input dessous).
- Les selects (salon, rôle) : `width: 100%` strict.
- Vérifier que le composant `ConfigFieldRow.vue` empile bien en mobile.
- Conteneur parent en `display: block` au lieu de `grid` multi-col.

---

## RbacPage — **catastrophe complète**

### 🔴 Section Utilisateurs / Rôles / Date d'attribution
Tableau utilisateurs déborde, la colonne date casse l'affichage.
**Fix** :
- Wrapper `overflow-x: auto` sur la table.
- En mobile : masquer la colonne date (`@media (max-width: 600px) { td:date { display: none } }`).
- Ou format date court (JJ/MM au lieu de JJ/MM/AAAA HH:MM:SS).

### 🔴 Section "Permissions sensibles" (ComponentMinRoleGrid) cassée
Le tableau action / role / floor / état / reset déborde et explose.
**Fix** :
- En mobile : convertir le tableau en cards verticales (1 row = 1 card).
- Ou masquer colonnes secondaires (floor, état) sous 600px.
- Le `<select>` du min_role doit avoir `width: 100%` en mobile (déjà fait dans le composant).

### 🔴 Section "Visibilité des composants" (ComponentVisibilityGrid) cassée
La grille actuelle a des colonnes fixes pour chaque rôle (viewer / mod / admin / owner) → explose en mobile.
**Fix demandé par user** :
- Mobile : empiler **verticalement** chaque composant.
- Layout pour chaque composant : nom du composant sur sa ligne, puis 4 boutons/checkboxes (`viewer / modérateur / admin / owner`) en dessous.
- En desktop garder la grille actuelle, switch via `@media (max-width: 768px)`.

---

## WalletPage — fix précédent insuffisant

> Partiellement fixée (commit `d0fa22c2`, padding/font réduits sous 480px), mais 2 zones encore cassées.

### 🔴 Section "Reset global" (danger zone) — tout sur 1 ligne
Affiche : icône ⚠️ + titre "Reset global" + texte explicatif + input "Nouveau solde" + bouton "🔥 Reset tout" — tous sur 1 ligne et **explose** en mobile.
**Fix** :
- En `@media (max-width: 600px)` : `flex-direction: column` sur `.danger-zone` + chaque enfant pleine largeur.
- L'icône au-dessus du titre, texte explicatif sur sa ligne, input + bouton stackés en dessous.

### 🔴 Liste des joueurs (tableau wallets) déborde encore en mobile
Visible en mobile : `# / Joueur / Solde / ...` puis colonnes suivantes coupées hors viewport.
Les breakpoints existants (1100px / 700px) masquent déjà certaines colonnes mais **pas assez sous 480px**.
**Fix** :
- Ajouter un breakpoint `@media (max-width: 480px)` qui réduit encore : ne garder que `Avatar / Joueur / Solde + actions`.
- Ou wrapper avec `overflow-x: auto` et garder toutes les colonnes accessibles via scroll.
- Idéalement : convertir le tableau en mini-cards en mobile (1 wallet = 1 card avec layout vertical).

---

## SlotPage

> Page principale fixée (commit `d0fa22c2`), mais le tableau des spins déborde.

### ✅ Layout général : propre.

### 🔴 Tableau "Spins récents" déborde du cadre
Colonnes visibles : `Heure / Joueur / Symboles / Mise` puis `Gain` est tronqué et s'il y a d'autres colonnes après elles ne sont plus visibles.
**Fix** :
- Wrapper du tableau avec `overflow-x: auto` pour scroll horizontal en mobile.
- Masquer colonnes secondaires (`Symboles` ou `Mise`) en `@media (max-width: 600px)` via `display: none`.
- Ou en mobile : convertir les lignes en mini-cards verticales (label / valeur key/value).

---

## BlackjackPage

### 🔴 Header — titre + sous-titre + boutons tous sur 1 ligne
`Blackjack — Administration des parties, surveillance historique, annulation et remboursement` + `↻ Rafraîchir` + `🗑 Reset total` → tous sur 1 ligne, explose le cadre.
**Fix** :
- Titre + sous-titre sur 1 ou 2 lignes en haut.
- Boutons en dessous (pleine largeur en mobile, alignés à droite en desktop).
- Container header en `flex-direction: column` avec `gap: 12px`.

### 🔴 Cards de parties (filtrées par statut "En cours / Victoire / Défaite / Annulé") cassées
Les cards en bas affichent `Joueur 15 / Statut / ...` mais le contenu est trop dense pour la largeur disponible → tout explose.
**Fix** :
- **Élargir les cards** (réduire le `gap` de la grille parente, ou passer en `auto-fit` minmax plus large).
- Réduire `margin` interne et `padding` qui prennent trop de place.
- En mobile : passer chaque card en pleine largeur (1 par ligne) avec layout vertical : joueur en haut, statut, montant, actions en bas.

---

## CoudePage (Coup de coude)

### 🔴 Header — titre + sous-titre + boutons tous sur 1 ligne
La barre du haut affiche `Coup de coude — Administration du jeu de survie` + boutons `↻ Rafraîchir` + `🗑 Reset total` tous alignés horizontalement → explose en mobile.
**Fix** :
- Titre + sous-titre sur 1 ligne (ou 2 lignes empilées) en haut.
- Boutons en dessous, en pleine largeur ou alignés à droite avec wrap.
- Container header en `flex-direction: column` avec `gap: 12px` en mobile.

### 🟠 Onglets (Combats / Stats joueurs / ...) risque de déborder
Selon le user, certains onglets ne sont peut-être pas tous visibles → manque potentiel d'onglets ou overflow horizontal.
**Fix** : `flex-wrap: wrap` ou `overflow-x: auto` sur la barre d'onglets pour scroll horizontal en mobile.

### ✅ Stats joueurs : propre.
### ✅ Onglet Social : propre.
### ✅ Onglet Tournoi : propre.

---

## GamesPage (Gestion des jeux) — cosmétique

> Pas de débordement, n'explose pas. Améliorations cosmétiques uniquement.

### ⚪ Cards de jeux à retravailler
Les cards sont propres mais **prennent trop de place disponible**.
**Améliorations souhaitées** :
- **Badges** (ex: "Survie", "FPS", "Gestion") : alignés tout à droite de la card, pas mélangés avec le titre.
- Repenser le layout : icône + titre + badge sur une ligne, description compacte en dessous, action en footer.
- Densifier (réduire padding, line-height) pour afficher plus de cards par ligne en grand viewport.

---

## AuditPage (Journal d'audit) — **catastrophe**

### 🔴 Cards d'événements explosent à cause des IDs longs
Exemple : `Changement de salon — Déplacement numéro de salon X vers numéro Y` → les snowflakes (17-19 chiffres) ne wrappent pas et débordent la card complètement.
**Cause probable** :
- Texte avec snowflakes en monospace inline sans `word-break: break-all` ou `overflow-wrap: anywhere`.
- Card avec `width` fixe ou `min-width` non contraint.
- Layout flex sans `min-width: 0` sur l'enfant texte.
**Fix — pistes** :
- **Simplification** : remplacer les snowflakes bruts par les noms (déjà résolus dans `audit_logs.channel_name` / `target_name`). Si ces champs sont peuplés, les afficher au lieu des IDs.
- **Word-break** : `overflow-wrap: anywhere` (ou `word-break: break-all` en fallback) sur le `.line .m` ou équivalent.
- **Layout card** : `min-width: 0` sur le bloc texte, `overflow: hidden` sur la card, contenu en colonne (icône + titre en haut, détails en bas).
- **Variante mobile** : tronquer le détail à N caractères + bouton "Voir détails" qui ouvre la modale `AuditEventDetail`.

---

## AutomodPage — Timeline des détections

> Page principale partiellement fixée (commit `d0fa22c2`), mais la barre de filtres timeline reste cassée.

### 🔴 Barre filtres timeline (titre + filter user_id + bouton) toute sur 1 ligne
Sur la section "Timeline des détections" : le titre, l'input `Filtrer par user_id` et le bouton à côté sont alignés horizontalement → explose.
**Fix demandé par user** :
- Titre **"Timeline des détections"** sur sa propre ligne, au-dessus.
- Input `user_id` : 1 ligne pleine largeur.
- Bouton (vraisemblablement "Filtrer" / "Reset") : 1 ligne pleine largeur en dessous, ou regroupé avec l'input à droite uniquement si la largeur le permet.
**Implémentation** : conteneur `.timeline-header` ou similaire en `flex-direction: column` + `gap: 8px`, ou utiliser `flex-wrap: wrap` avec contraintes `min-width: 0` sur les enfants.

---

## SecurityPage (Événements de sécurité) — empty state

> Page partiellement fixée précédemment (commit `d0fa22c2`), mais l'**empty state** déborde encore.

### 🔴 Cadre "Aucun événement de sécurité" déborde du téléphone
La barre de recherche en haut est OK ✓.
Juste en dessous, le bloc empty state (composant `EmptyState` ?) déborde de l'écran sur mobile.
**Cause probable** :
- `min-width` fixe sur le conteneur de l'empty state.
- Ou padding/margin trop grand combiné à `width: 100%` sur un parent qui n'a pas `box-sizing: border-box`.
- Ou icône SVG/illustration de l'empty state avec `width` fixe (ex: 200px) sans `max-width: 100%`.
**Fix** :
- Inspecter `EmptyState.vue` (atom) → vérifier que sa largeur est contrainte (`max-width: 100%`).
- Sur la card parent : `min-width: 0` + `box-sizing: border-box`.

---

## TempRolesPage (Rôles temporaires)

### 🔴 Formulaire — user ID / role ID sur la même ligne
Les inputs `user ID` et `role ID` sont côte à côte → explosent en mobile.
**Fix demandé par user** : 1 input par ligne, pleine largeur.
**Implémentation** : `flex-direction: column` (ou `grid-template-columns: 1fr`) sur le conteneur du formulaire + chaque input `width: 100%`.

> Note : si la page existe encore, profiter du nouveau composant `IdMultiplierMapField` (`temp_roles` est déjà câblé en `kind: role` avec `valueLabel: "Durée (s)"`) — peut remplacer le formulaire manuel par un picker rôle propre.

---

## SponsorshipsPage (Parrainage)

### 🔴 Formulaire "Nouveau parrainage" — sponsor / sponsored sur la même ligne
Les deux inputs `sponsor` et `sponsored` (ou équivalent) sont alignés horizontalement et **explosent** en mobile.
**Fix demandé par user** : 1 input par ligne, pleine largeur, peu importe le viewport.
**Implémentation** : `flex-direction: column` (ou `grid-template-columns: 1fr` au lieu de `1fr 1fr`) sur le conteneur du formulaire + chaque input `width: 100%`.

---

## LevelsPage (Niveaux / XP)

> **C'est un carnage.** Plusieurs sections cassées en mobile, à reprendre largement.

### 🟠 Petits rectangles/cards stats en haut
Les cartes prennent **trop de place** en mobile.
**Options de fix** :
- Soit les **élargir** sur 1 ligne pleine largeur (1 card par ligne en très petit écran).
- Soit les **rétrécir** drastiquement (font-size réduit, padding minimal) pour 2-3 par ligne.
- Probablement mieux : `grid-template-columns: repeat(auto-fit, minmax(120px, 1fr))` + padding réduit en `@media (max-width: 480px)`.

### 🔴 Combobox "Mode d'attribution des rôles" / "Total XP" déborde
Le `<select>` du mode d'attribution dépasse complètement du cadre.
**Cause probable** : `min-width` fixe sur le select ou pas de `width: 100%` + `min-width: 0` sur le parent.
**Fix** : `width: 100%` sur le select, `min-width: 0` sur le wrapper flex.

### ✅ Onglets "Global / Texte / Vocal"
Tabulation propre, **rien à toucher**.

### 🔴 Carte joueur (classement / roll par niveau) **complètement explosée**
Tout se superpose, rien ne s'aligne, elle est cassée en mobile.
**Pistes** :
- **Simplifier** la card : moins d'infos visibles en mobile, le détail va dans une vue dépliable / modale.
- En desktop la garder dense, en mobile passer en layout vertical empilé : avatar en haut, nom, niveau, XP bar en pleine largeur.
- Enlever les colonnes secondaires (rang, % progression) en mobile via `display: none`.

### 🔴 Onglet "Rôle par niveau" — header pété
La barre du haut affiche `Mode attribution par rôle / Total XP texte / etc.` tous alignés et déborde.
**Fix** : empiler verticalement les contrôles, ou les regrouper dans un encadré "Paramètres" repliable.

### 🔴 Convertisseur niveau ↔ XP cassé
Affichage `Niveau 1` OK mais le label `Texte cumulé / etc.` à côté **explose**.
**Cause probable** : flex row sans wrap, labels longs.
**Fix** : `flex-direction: column` sur le bloc convertisseur en mobile, ou mettre le label au-dessus de la valeur (layout key/value vertical).

### 🔴 "Définition de niveau / texte et vocal requis" + recherche rôle sur la même ligne
Le bloc explicatif (`Définition de niveau de texte et de vocal requis...`) et la recherche de rôle sont **sur la même ligne** → débordement.
**Fix demandé par user** :
- Texte d'explication : 1 ligne pleine largeur, en haut.
- Recherche rôle : 1 ligne pleine largeur, juste en dessous.
- Donc `flex-direction: column` sur le conteneur parent + chaque enfant `width: 100%`.

---

## DiscordRolesPage (Rôles & Auto-rôles)

### 🔴 Vue principale (avant clic "Voir tous les rôles") complètement cassée
Le header de la page contient :
- Titre
- Bouton **"+ Nouveau rôle"**
- Bouton **"Voir tous les rôles Discord"**

Tout est aligné sur **une seule ligne** et explose visuellement (chevauchements, débordement).
**Fix** :
- Header en `flex-direction: column` pour empiler titre / actions.
- Ou `flex-wrap: wrap` avec `gap` cohérent + actions sur une 2ème ligne.
- En mobile : actions en pleine largeur, stackées.

### 🟠 Vue "tous les rôles Discord" — recherche + bouton "Créer un rôle"
**Cartes des rôles** : OK, propres, pas de souci. ✓
**Barre de recherche** + bouton **"Créer un rôle"** juste à côté : le bouton **dépasse** du conteneur.
**Cause probable** : input `flex: 1` sans `min-width: 0` ou bouton avec `min-width` fixe trop grand, sans `flex-shrink`.
**Fix** :
- `min-width: 0` sur l'input.
- `flex-shrink: 0` sur le bouton (mais c'est déjà OK), réduire son padding en mobile.
- En `@media (max-width: 600px)` : empiler input + bouton verticalement.

---

## VoiceChannelsPage

> Cassée également à grand viewport, pas spécifique au mobile.

### 🔴 Cartes KPI en haut (Total / Public / Privé) débordent du cadre
Les cartes statistiques en haut de page sortent du conteneur parent.
**Cause probable** : grille KPI fixe (`repeat(N, 1fr)` ou `minmax(...)` trop grand) sans contrainte sur le parent, ou les cards ont un `min-width` qui force le débordement.
**Fix** : `grid-template-columns: repeat(auto-fit, minmax(140px, 1fr))` sur le conteneur KPI + `min-width: 0` sur les cards.

### 🔴 Tableau "Salons VOS / Public / Privé" déborde
Le tableau principal des salons actifs dépasse complètement la largeur du conteneur.
**Cause probable** : colonnes en largeur fixe ou `grid-template-columns` somme > 100%, pas de `overflow-x: auto` sur le wrapper, pas de `min-width: 0` sur le parent.
**Fix** :
- Wrapper du tableau avec `overflow-x: auto` pour scroll horizontal propre.
- Réduire les colonnes "secondaires" (ex: nb users, ratios) en mobile via media query `@media (max-width: 900px)` qui les masque (`display: none`).
- Privilégier des grilles `auto` au lieu de largeurs absolues.

### 🔴 Pagination déborde
La barre de pagination sous le tableau dépasse également l'écran.
**Cause probable** : `display: flex` avec trop de boutons (Précédent, Suivant, numéros 1-2-3-4-5...) et pas de `flex-wrap` ni de bouton "..." pour les longues séries.
**Fix** :
- Composant `PaginationBar` à revoir : limiter à 3-5 numéros visibles + ellipsis pour le reste.
- En mobile : afficher uniquement Précédent / `Page X / N` / Suivant (pas de numéros).

### 🟠 Inputs (filtres / recherche)
Les inputs de filtre prennent toute la place disponible **et c'est OK** (1 ligne propre par input). Pas à modifier — fonctionne bien selon le user.

### 🔴 Tableau historique en dessous + sa pagination
Idem que le tableau principal : déborde + pagination déborde dans l'onglet "Historique".
**Fix** : mêmes solutions que ci-dessus (overflow-x, masquer colonnes secondaires, simplifier la pagination).

---

## Pages déjà fixées (commit `d0fa22c2`)

Ne pas re-traiter, déjà OK sous 640px :

- ✅ AutomodPage (kpi-row 3→1, grid 2→1)
- ✅ SlotPage (kpi-row 4→2, grid 2→1)
- ✅ WheelPage (kpi-row 3→1, grid 2→1)
- ✅ LevelsConfigPage (grid + form 2→1)
- ✅ WalletPage (padding/font réduits sous 480px)
- ✅ RbacPage (input min-width:0, table overflow)
- ✅ MembersPage (sort-select width:100%)
- ✅ SecurityPage (stats-grid minmax 120px, search-input clamp)

---

## À auditer

- [ ] **CoudePage** : tableaux des combats / leaderboard joueurs probablement larges.
- [ ] **BlackjackPage** : grille des parties.
- [ ] **AnnouncementsPage** : modale et chips multipicker (refait récemment).
- [ ] **ConfessionsPage** : moderation page complète.
- [ ] **ConfigFieldRow** dans ComponentConfigPage : alignement label / input multi-colonnes.
- [ ] **TopBar** : si plusieurs guilds, dropdown + avatar + logo serrés.
- [ ] **ServerHealthPage** : graphiques (Chart.js) qui peuvent overflow.
- [ ] **ServerSecurityPage** : tableaux fail2ban, nombreuses colonnes.
- ~~**VoiceChannelsPage**~~ → documentée plus haut (KPI cards + 2 tableaux + paginations).
- [ ] **AiDatasetPage** : table de messages.

---

## Notes méthodo

- Tests à faire : Chrome DevTools mode iPhone SE 375×667, iPhone 12 Pro 390×844, Galaxy S20 360×800.
- Viewports prioritaires : **375px** (le plus serré largement utilisé).
- `MainLayout` réduit déjà le padding global à 14px×10px sous 600px.
- Les nouveaux composants (IdMultiplierMapField, IdsListPickerField) ont des `@media (max-width: 600px)` natifs.
- La règle CSS globale dans `global.css` pour les checkboxes ne pose pas de souci responsive.

---

## Prochaines étapes proposées

1. Fix prioritaire **ModerationHubPage** (3 issues confirmées) :
   - Onglets : scroll horizontal avec masque/gradient.
   - Inputs : stacker en colonne.
   - KPI label : ellipsis.
2. Audit page par page de la liste "À auditer".
3. Test final sur device réel.
