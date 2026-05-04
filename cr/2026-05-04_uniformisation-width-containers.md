# CR — Uniformisation de la max-width des containers de page — 2026-05-04
**Présents** : Clara (PO), Léa (Architect), Inès (Frontend), Tom (QA)
**Rédigé par** : Clara (Product Owner)

## Décisions
- Width par défaut des pages : **1280px** (renomme l'ancien `.page--constrained`).
- Deux exceptions nommées et documentées :
  - `.page--narrow` (480px) — auth, onboarding, modales pleine page.
  - `.page--wide` (1920px) — dashboards et analytics denses.
- `.page--xs` (720px) et `.page--sm` (960px) **supprimés**, mais seulement après audit des usages restants.
- Toute racine de page DOIT porter une des trois classes officielles.
- Aucune `max-width` en dur sur un selecteur de page : règle stylelint à mettre en CI.
- Un ADR court trace la décision et les alternatives ecartees.

## Questions ouvertes
- Faut-il aussi normaliser les `max-width` internes des organisms (cards analytics, listes) ou rester au niveau page ?
- Le default 1280 s'applique-t-il a la webview Tauri sans ajustement ?
- Le 1280 est-il `max-width` du contenu ou inclut-il les paddings horizontaux ?

## Risques identifies
- Regressions visuelles sur les pages qui s'appuyaient sur 720/960 (espace blanc excessif).
- Grids analytics calibrees pour 1280 qui passent mal en `--wide` 1920.
- `max-width` en dur dans des composants enfants qui survivront au refactor.
- Laptops 1366px : container 1280 + paddings peuvent generer du scroll horizontal.

## Actions
- [ ] Audit des usages de `.page--xs` et `.page--sm` + liste des `max-width` hardcodes sur racines de page — owner : Ines
- [ ] ADR (1 page) actant les 3 classes officielles et l'interdiction des valeurs en dur — owner : Lea
- [ ] Migration page par page vers les 3 classes, suppression de `--xs`/`--sm` une fois a zero usage — owner : Ines
- [ ] Regle stylelint interdisant `max-width` en dur sur les selecteurs `.page-*` — owner : Ines
- [ ] Checklist visuelle avant/apres en 1366, 1440, 1920 sur les pages migrees — owner : Tom
- [ ] Validation que `narrow` et `wide` couvrent bien des parcours metier, pas des preferences dev — owner : Clara
