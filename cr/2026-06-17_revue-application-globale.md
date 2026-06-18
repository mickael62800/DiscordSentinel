# CR — Revue d'ensemble de l'application — 2026-06-17
**Présents** : Clara (Product Owner), Léa (Architecte), Rachid (Sécurité), Hugo (DevOps/SRE), Tom (QA), Léo (Community Manager Discord)
**Rédigé par** : Clara (Product Owner — chef de projet IT)

## Contexte
Revue holistique de DiscordSentinel (workspace Rust hexagonal : bot Serenity,
API Axum, core, worker, proto gRPC + web Vue). Le produit est riche et cohérent
fonctionnellement. Objectif : identifier ce qui manque d'utile, hors features.

## Décisions
- Backups Postgres automatisés ET testés (restore réel vérifié), chiffrés —
  priorité haute. Protège l'existence des données (wallets, historiques, traces).
- Réparer la suite de tests + CI bloquante, SANS geler les features : un sprint
  de remise à zéro, puis règle « chaque feature couvre son use case ».
- Rétention + effacement : RECOMMANDÉ (hygiène), pas obligatoire pour un serveur
  privé/entre potis (exemption RGPD « activité personnelle/domestique »).
  Le RGPD ne devient réellement applicable que si le serveur est PUBLIC /
  communautaire UE. Dans tous les cas : purge par type de donnée (IP de login,
  messages, transcripts, user_activity, logs) + effacement ciblé par membre —
  utile par simple hygiène même hors obligation légale.
- Durcir les tokens : chiffrement at-rest au niveau volume + backup chiffré en
  1re étape ; chiffrement/hachage dédié des refresh tokens (longue durée) ensuite.
- Vue web « dossiers résolus / historique membre » : la trace est déjà en DB,
  il manque l'UI de consultation.
- Observabilité 2e étage : alerting + quelques SLO sur les métriques Prometheus
  déjà exposées.

## Questions ouvertes
- Sprint dédié aux tests vs rattrapage continu : acceptable côté delivery ?
- Durées de rétention par type de donnée (ex. messages 30j, transcripts 1 an, logs 90j) ?
- Chiffrement des tokens : volume-level suffisant, ou chiffrement applicatif des refresh tokens requis ?
- Flux d'appel DSA réellement bouclé (canal d'appel + réponse au membre) ou seulement mentionné ?
- Versioning de contrat proto/HTTP + tests de contrat (Pact / snapshots OpenAPI) ?

## Risques identifiés
- Perte de données irréversible : pas de backup testé → incident DB ou migration ratée efface tout.
- Régressions sanction/argent : suite de tests cassée → bug sur mute/ban/wallet en prod sans alarme.
- Fuite de tokens : DB compromise → access/refresh tokens en clair rejouables (impersonation).
- Accumulation indéfinie de messages/PII/IP sans purge (risque RGPD seulement si
  serveur public ; sinon simple mauvaise hygiène + surface de fuite accrue).
- Désync bot↔API silencieuse sur un changement de contrat non versionné.

## Actions
- [ ] Backup Postgres chiffré + test de restore réel ; chiffrement at-rest volume ; alerting sur métriques clés — owner : Hugo
- [ ] Remettre la crate de tests au vert (stubs manquants) + CI bloquante ; amorcer un test de contrat bot↔API — owner : Tom
- [ ] Threat model tokens ; spéc chiffrement/hachage des refresh tokens ; scan secrets + cargo audit en CI ; purge de rétention (hygiène, obligatoire seulement si serveur public) — owner : Rachid
- [ ] Page web historique / dossiers résolus + définir 3 KR d'usage (cartes traitées, taux d'appel, rétention) — owner : Clara
- [ ] Vérifier/boucler le flux d'appel DSA (canal + réponse) + vue historique membre — owner : Léo
- [ ] Proto/HTTP versionné + test d'archi « bot ne dépend pas de core » — owner : Léa
