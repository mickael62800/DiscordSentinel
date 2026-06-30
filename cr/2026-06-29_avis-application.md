# CR — Avis sur l'application — 2026-06-29
**Présents** : Clara (Product Owner), Léa (Architect), Marc (Backend), Inès (Frontend), Tom (QA), Rachid (Security)
**Rédigé par** : Clara (Product Owner)

## Contexte
Réunion d'évaluation globale de DiscordSentinel à l'issue d'une session de remise en état (suite de tests réparée ~660 erreurs -> 0, DRY, code mort supprimé, mojibake corrigé, god files découpés, bug d'auth front trouvé).

## Décisions
- Architecture conservée : hexagonal côté Rust (domain pur dans sentinel-core), Atomic Design côté Vue. Structure jugée saine, pas de refonte.
- Garde-fous CI obligatoires et désormais verts localement : clippy -D warnings, fmt, cargo check, tests core ; build + lint web. À rendre bloquants en PR.
- Tout appel réseau métier du front doit passer par http.ts / services (zéro fetch brut). Bug AddWatchModal (POST sans headers d'auth) corrigé.
- ESLint installé en mode garde-fou (warnings sémantiques, cosmétique désactivé), durcissement progressif, règles sécurité à passer en error.
- Politique anti-god-file : pas de composant Vue > ~600 lignes sans justification. MemberDetailDrawer / ModerationJournalTab / DockerAdminSection / AnnouncementFormModal déjà découpés.

## Questions ouvertes
- Usage réel par feature (modération, automod, coude, casino, blackjack, tickets, voice, tamagotchi, progression) : aucune instrumentation aujourd'hui -> arbitrage de scope impossible.
- Tests combat probabilistes (RNG from_entropy) : assertions statistiques sur N runs ou job nightly isolé plutôt que seed figée.
- Couverture front : 10 tests pour 186 composants — niveau cible à définir.
- SQL en chaînes runtime (non vérifié à la compilation) : adopter sqlx::query! sur les requêtes critiques ?

## Risques identifiés
- Régression SQL silencieuse : un JOIN oublié dans une requête en chaîne casse en prod, pas en CI (cf. réécriture LATERAL de watched_user_repository).
- Auth contournée : pattern de fetch brut côté front à éradiquer complètement (AddWatchModal n'était peut-être pas le seul historiquement).
- Tests flaky ignorés : un rouge intermittent désensibilise l'équipe et rend la suite décorative.
- Supply chain : 3 vulnérabilités npm "high" non traitées, pas de cargo audit / SBOM en CI.
- Dérive sans garde-fou : on a réparé l'état sans verrouiller la CI ; sans cela, re-dégradation à 6 mois.

## Actions
- [ ] Brancher les tests d'intégration api/bot en CI via testcontainers (Postgres+Redis) — owner : Tom — échéance : prochain sprint
- [ ] Corriger le RBAC manquant sur l'endpoint announcements (vérif admin) — owner : Rachid — échéance : cette semaine
- [ ] npm audit fix + ajouter règles ESLint sécurité en error — owner : Rachid
- [ ] Auditer les sous-requêtes corrélées restantes ; évaluer sqlx::query! sur requêtes chaudes — owner : Marc
- [ ] Grep des fetch( métier restants -> router via services ; réduire les 34 any restants — owner : Inès
- [ ] Instrumenter 3 KR d'usage par grande feature avant tout arbitrage de scope — owner : Clara
- [ ] ADR court "garde-fous CI" + test d'archi (dependency boundaries) côté front — owner : Léa
- [ ] Décider du traitement des tests combat flaky (stats vs nightly) — owner : Tom + Marc
</content>
