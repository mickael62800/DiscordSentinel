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
- [ ] Définir les catégories "auto" (phishing/raid) + seuils, avec réversibilité, log et appel — owner : Ava
- [x] Auditer l'idempotence des 3 chemins de finalisation et le log-dans-la-transaction — owner : Marc
  - Vote → Finaliser : OK (gate DB `/resolve` sur status pending|decided, 2e clic = Conflict avant apply/log).
  - Web-resolve : gate DB OK, mais redelivrance Redis possible → AJOUT d'un verrou anti-redelivrance (claim par review).
  - 1-clic : trou reel (applique la sanction sans gate DB, bouton sans review_id) → AJOUT d'un verrou d'idempotence en memoire au niveau carte (une action par carte).
  - Reste recommande : (1) log de sanction dans la MEME transaction API que la resolution (aujourd'hui 2 appels separes cote bot) ; (2) le verrou memoire ne couvre pas un deploiement multi-process/sharding — un verrou DB serait necessaire a cette echelle.
- [ ] Spécifier le contrat API should_card / aggregate_into et retirer le routage redondant du bot ; vérifier les rate limits d'édition — owner : Kenji
- [ ] Rédiger un ADR court "décider = API / agir = bot" (frontière + périmètre proto) — owner : Léa
- [ ] Trancher la règle de split de carte agrégée et le drapeau membre vulnérable — owner : Nora
- [ ] Valider les gabarits de message de sanction (ton + mention systématique du droit d'appel), y compris actions automatiques — owner : Léo
