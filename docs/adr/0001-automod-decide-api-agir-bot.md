# ADR 0001 — Automod : décider = API, agir = bot

- **Statut** : Accepté (2026-06-17)
- **Auteur** : Léa (architecte)
- **Portée** : pipeline de modération automatique (sentinel-bot ↔ sentinel-proto ↔ sentinel-api/core)

## Contexte

`automod-bot` ne dépend que de `sentinel-proto` (jamais de `sentinel-core`). Au
fil des features (modération humaine, auto-protection, suppression de liens,
cartes de vote), la **décision de routage** d'une détection — faut-il poster une
carte ? appliquer une action auto ? ne rien faire ? est-ce un cas sévère ? un
lien à supprimer ? — s'était retrouvée **dupliquée dans le bot** : le bot lisait
la config guild (`human_only`, `ai_review_mode`, `review_min_score`,
`auto_protect_enabled`, `auto_delete_links_enabled`, `log_channel_id`, …) et
recalculait la décision.

Problèmes : règle dupliquée bot/web (risque de divergence), config métier lue
dans un adapter Discord, logique difficile à tester (couplée à Serenity), et un
contrat gRPC qui ne portait que `action/score/reason`.

## Décision

Séparer **décider** et **agir** :

- **Décider = API.** La règle de routage vit dans le domaine pur
  `sentinel-core::domain::services::moderation::automod_routing::decide`, appelée
  par `AnalyzeMessageService` (qui charge déjà la config guild). Elle produit une
  `RoutingDecision { route: None|Card|Auto, severe, auto_delete_link }`.
- **Contrat = proto.** `AnalyzeMessageResponse` expose `route` (enum `Routing`),
  `severe`, `auto_delete_link` en plus de `action/score/reason`.
- **Agir = bot.** `send_to_backend` ne décide plus : il **exécute** la décision
  (`match response.route`). Le bot garde uniquement ce qui est intrinsèquement
  Discord (effets : mute/ban/suppression, rendu des cartes, DM) et un fallback
  minimal si l'API est injoignable (`human_only` → ne rien faire).

Règle générale : **toute décision de politique de modération est calculée
côté serveur ; le bot n'est qu'un exécuteur d'effets Discord.**

## Conséquences

**Positif**
- Une seule source de vérité pour le routage (bot et web cohérents).
- Règle pure, testable sans I/O ni Serenity.
- Le bot s'allège (5 paramètres de config supprimés de `send_to_backend`).

**Négatif / coûts**
- Le proto devient le contrat critique : toute nouvelle entrée de décision = champ
  proto + **redéploiement coordonné API + bot**.
- Hot path (un appel par message) : la décision est calculée à chaque analyse
  (négligeable — config déjà chargée, fonction pure).

**Limites assumées**
- Le **flood** reste décidé côté bot (compteur en mémoire, pas connu de l'API) ;
  il déclenche l'auto-protection localement puis suit le flux normal.
- L'**agrégation** (`aggregate_into`) reste côté API via `create_or_merge`
  (flag `merged`), elle n'a pas eu besoin de remonter dans `analyze`.

## Alternatives écartées

- **Tout garder dans le bot** : statu quo → duplication bot/web, intestable.
- **Nouvel endpoint HTTP de routage** : 2e aller-retour sur le hot path le plus
  chaud du projet ; rejeté pour la latence. La réponse gRPC existante suffit.
- **Transaction ACID unique resolve+log** (sujet voisin) : jugée disproportionnée
  (variantes `_tx` sur audit_logs/strike/automod) ; traitée séparément par un log
  côté serveur dans la même requête, sans single-tx.

## Implémentation

- `sentinel-core/src/domain/services/moderation/automod_routing.rs` (règle pure)
- `sentinel-core/.../ai/analyze_message_service.rs` (calcul + `MessageAnalysis`)
- `sentinel-proto/proto/automod.proto` (`Routing` + champs de réponse)
- `sentinel-bot/.../automod/{backend.rs,message_handler.rs,api_client.rs}` (exécution)
