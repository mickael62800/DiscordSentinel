# CR — Revue de la nouvelle modération automod — 2026-06-17
**Présents** : Léo (Community Manager), Nora (Mod Lead), Ava (Trust & Safety), Kenji (Admin technique), Marc (Backend), Léa (Architecte)
**Rédigé par** : Léo (Community Manager — chef de projet Discord)

## Contexte
Revue de la modération automod après la session d'évolutions : modération humaine
par cartes (vote + finalisation), clore/rouvrir, auto-protection réversible
(phishing/invitation/gros flood), suppression auto des liens non-image, traçabilité
+ DM droit d'appel, routage décidé côté API (decide=API/agir=bot), gabarits de
sanction, verrous d'idempotence, log de sanction côté serveur, agrégation par
utilisateur. Verdict général : base saine et cohérente ; restent des angles morts
d'observabilité, de fiabilité worker, et un point de politique sur les liens.

## Décisions
- Lien générique (`flags.link` hors phishing/invitation) → CARTE, plus de suppression
  sèche. Phishing / invitation Discord / raid restent en auto. Rendre la suppression
  auto des liens configurable.
- Ajouter une notice membre à la finalisation de vote (sanction motivée + droit
  d'appel), pour aligner le ton avec les autres chemins.
- Observabilité d'abord : instrumenter les décisions automod (compteurs carte/auto/
  suppression, sévères, appels, logs de sanction émis vs attendus) avant d'ajouter
  de la plomberie transactionnelle.
- Fiabiliser le worker `/decide` : healthcheck + balayage de rattrapage des cartes
  dont l'échéance est dépassée.
- Outbox de sanction retenu sur le principe, mais conditionné à un compteur
  « logs manquants » non nul (sinon sur-ingénierie).

## Questions ouvertes
- Format de la carte « lite » pour les liens génériques (nouveau format léger vs carte de vote).
- Où exposer les métriques automod (page web, logs système, Prometheus existant ?).
- Verrou d'idempotence : rester en mémoire (mono-instance) ou passer à un verrou DB (advisory lock) dès maintenant ?
- Fallback quand le DM de droit d'appel échoue (MP fermés) : mention en salon de discussion, ou rien ?
- Le ban auto reste une « proposition » (pas de ban réel) — clarifier le libellé ou le comportement.

## Risques identifiés
- Worker `/decide` muet → cartes de vote jamais finalisées, sanctions en suspens.
- Suppression de liens légitimes (faux positifs) sans visibilité modo → perte de confiance.
- Escalade faussée si un log de sanction est perdu (resolve committé sans log).
- Rate limit Discord sur l'édition de carte agrégée lors d'un gros flood → carte figée/incohérente.
- Multi-instance futur : double sanction (verrou d'idempotence en mémoire non partagé).

## Actions
- [x] Healthcheck worker + rattrapage des cartes en retard ; audit rate limits — owner : Kenji
  - Rattrapage DÉJÀ couvert par le job `automod_close_votes` (worker, sweep idempotent toutes les 60s : `SELECT ... status='voting' AND voting_deadline < NOW() LIMIT 50` → POST /decide). S'il était down, il rattrape au redémarrage (50/tick). Heartbeat worker déjà en place (`start_heartbeat`).
  - Édition de carte agrégée : `edit_aggregated_card` n'édite que l'embed et ne recrée pas sur erreur transitoire → pas d'amplification. Audit OK, pas de changement nécessaire pour l'instant.
- [x] Métriques automod — owner : Ava
  - `automod_decisions_total{route,severe,link_delete}` (handler gRPC analyze) et `automod_sanction_log_total{result=ok|error}` (resolve serveur). Scrapés via `/metrics`.
  - Mode shadow : reporté (non bloquant ; à faire si réglage de seuils nécessaire).
- [x] Lien générique → carte — owner : Nora
  - `automod_routing::decide` : lien générique (hors phishing/invitation, hors image) part en CARTE par défaut. Suppression sèche = opt-in explicite (`auto_delete_links_enabled`, défaut passé à false, migration 274).
  - Carte « lite » : reportée — on réutilise la carte de review standard pour l'instant.
- [x] Notice membre de finalisation de vote — owner : Léo
  - DM au membre à la finalisation (prevention/warn/mute/ban) avec le gabarit `sanction_notice` (motif + droit d'appel). Best-effort.
- [~] Outbox de sanction — owner : Marc
  - REPORTÉ et CONDITIONNÉ : on instrumente d'abord `automod_sanction_log_total{error}`. Outbox (table + worker rejouant le log) à implémenter seulement si ce compteur est non nul en prod.
- [x] Verrou idempotence : DB vs mémoire — owner : Léa
  - DÉCISION : on RESTE en mémoire (mono-instance actuel). resolve/finalize/web sont déjà gardés au niveau DB (transition de statut) ; seul le 1-clic dépend du verrou mémoire. Passage à un `pg_advisory_lock` à reconsidérer le jour du multi-instance/sharding (noté comme dette).
