# CR — Modération, automod et cartes warn/ban — 2026-06-17
**Présents** : Léo (Community Manager), Nora (Mod Lead), Ava (Trust & Safety), Kenji (Admin technique / bot dev), Marc (Backend), Léa (Architecte)
**Rédigé par** : Léo (Community Manager — chef de projet Discord)

## Contexte
Automod en mode scoring (pas de ban automatique). Routage vers une carte de vote
dans le salon de review selon la config (`human_only` ou `ai_review_mode && score>=min`).
Agrégation par utilisateur (anti-flood), boutons Prévention/Warn/Suppression/Mute/Ban/
Ignorer + Ouvrir une discussion, finalisation par un modo, log de la sanction validée
dans l'historique. Commande `/signalement` pour carte manuelle avec contexte avant/après.

## Décisions
- La modération humaine par carte reste le mode par défaut pour tout ce qui demande
  du jugement (insulte, ton, conflit).
- Exception par catégorie de menace : phishing avéré et raid/mass-join déclenchent une
  action automatique (quarantaine / suppression / ban). `human_only` ne s'applique pas
  à ces catégories.
- Toute sanction, y compris automatique, est loggée et contestable (droit d'appel) —
  conformité DSA.
- Frontière clarifiée "décider = API / agir = bot" : l'API renvoie suggested_action,
  should_card et la cible d'agrégation ; le bot se contente d'exécuter.
- Idempotence garantie sur la finalisation et l'agrégation (advisory lock + dédup
  d'event Redis) ; le log de sanction est écrit dans la même transaction que
  l'application de la sanction.

## Questions ouvertes
- Seuil/score séparant "phishing avéré" (auto) de "lien suspect" (carte) : liste de
  domaines + heuristique à définir.
- Carte agrégée mélangeant un cas mineur et un cas grave : faut-il un split automatique ?
- Vue "dossier membre" lisible côté web (timeline des sanctions validées) distincte de
  la carte agrégée Discord ?
- Périmètre minimal de migration de la règle de routage vers le core via gRPC, sans
  faire exploser le contrat proto.
- Mineurs / membres vulnérables : drapeau spécifique sur la carte modifiant la graduation ?

## Risques identifiés
- Latence sur raid/phishing si `human_only` reste global (fenêtre ~10 min avant action humaine).
- Double sanction si l'idempotence n'est pas verrouillée sur les 3 chemins (vote, 1-clic, web-resolve).
- Carte agrégée illisible quand un user accumule trop d'incidents (cas grave noyé dans le bruit).
- Rate limits Discord sur l'édition répétée de la carte agrégée et la création de salons de discussion.
- Divergence de comportement bot/web tant que la décision n'est pas centralisée (règle dupliquée).

## Actions
- [x] Définir les catégories "auto" (phishing/raid) + seuils, avec réversibilité, log et appel — owner : Ava
  - Auto-protect (mute réversible + carte) pour phishing / invitation Discord / gros flood (migration 271).
  - Sanction auto désormais TRACÉE dans l'historique de modération (acteur = AutoMod, compte dans l'escalade).
  - Membre notifié en DM (motif + droit d'appel via /appeal) — conformité DSA (auto_protect_notify_member).
  - Liens non autorisés HORS image supprimés automatiquement + traçabilité (auto_delete_links_enabled, migration 272).
- [x] Auditer l'idempotence des 3 chemins de finalisation et le log-dans-la-transaction — owner : Marc
  - Vote → Finaliser : OK (gate DB `/resolve` sur status pending|decided, 2e clic = Conflict avant apply/log).
  - Web-resolve : gate DB OK, mais redelivrance Redis possible → AJOUT d'un verrou anti-redelivrance (claim par review).
  - 1-clic : trou reel (applique la sanction sans gate DB, bouton sans review_id) → AJOUT d'un verrou d'idempotence en memoire au niveau carte (une action par carte).
  - Log de sanction désormais écrit CÔTÉ SERVEUR dans la requête `/resolve` (helper `log_review_sanction`), avec les mêmes broadcasts (moderation_action / strike_added). Le bot ne fait plus de 2e appel HTTP (finalize + web-resolve) → fin de la fenêtre "résolu mais non loggé".
  - Limites restantes (assumées) : (1) ce n'est pas une seule transaction ACID (2 commits DB dans une même requête API) — un vrai single-tx exigerait des variantes `_tx` sur les repos audit_logs/strike/automod, jugé disproportionné/risqué sans tests ; (2) le verrou d'idempotence est en mémoire → un déploiement multi-process/sharding nécessiterait un verrou DB ; (3) le 1-clic continue de logger côté bot (il ne passe pas par `/resolve`).
- [x] Spécifier le contrat API should_card / aggregate_into et retirer le routage redondant du bot — owner : Kenji
  - La DÉCISION de routage est calculée côté serveur dans `analyze` (qui a déjà la config guild) : nouveau domaine pur `automod_routing::decide` → `RoutingDecision { route (None/Card/Auto), severe, auto_delete_link }`.
  - Contrat exposé via le proto `AnalyzeMessageResponse` : enum `Routing` + champs `route`, `severe`, `auto_delete_link`.
  - Le bot ne décide plus : `send_to_backend` exécute la décision serveur. Retrait du routage dupliqué + des helpers `is_severe_content`/`contains_non_image_url` côté bot, et de 5 paramètres de config devenus inutiles. `human_only` conservé uniquement pour le fallback "backend injoignable".
  - Agrégation (`aggregate_into`) : déjà côté API via `create_or_merge` (flag `merged` renvoyé) — inchangé.
  - Reste optionnel : vérifier les rate limits d'édition de la carte agrégée (non bloquant).
- [x] Rédiger un ADR court "décider = API / agir = bot" (frontière + périmètre proto) — owner : Léa
  - `docs/adr/0001-automod-decide-api-agir-bot.md` (statut Accepté) : contexte, décision, conséquences, limites (flood bot-side, agrégation API), alternatives écartées (tout-bot, endpoint HTTP dédié, single-tx ACID).
- [x] Split de carte agrégée + drapeau membre vulnérable/mineur — owner : Nora
  - Drapeau mineur/vulnérable : SANS OBJET — aucun mineur ni membre vulnérable sur le serveur (confirmé 2 fois par le propriétaire). Rien à implémenter.
  - Split de carte agrégée : laissé optionnel (non demandé), à rouvrir seulement si le flood agrégé devient illisible en pratique.
- [x] Valider les gabarits de message de sanction (ton + mention systématique du droit d'appel), y compris actions automatiques — owner : Léo
  - Gabarit unique `shared::embeds::sanction_notice(action, raison, durée, validé_par, appel)` : ton cohérent (titre/emoji/couleur par action) + champ « Contestation » → `/appeal` systématique.
  - Branché sur les messages membre : automod auto (`execute_action` : warn/delete/mute + ban), review 1-clic (`handle_review_button`), analyse d'image. Les actions auto-protect notifient déjà en DM avec l'appel.
  - Toggle web `sanction_appeal_enabled` (défaut true, migration 273) pour désactiver si le module d'appel n'est pas utilisé.
  - Reste optionnel : notice membre côté finalisation de vote (aujourd'hui silencieuse en salon ; le membre subit l'effet mais sans embed dédié).
