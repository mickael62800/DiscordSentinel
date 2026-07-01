# CR — Idées nouvelles features (ce qui ne se fait nulle part) — 2026-07-01

**Présents** : Léo (Community Manager), Kenji (Admin technique / bot dev), Théo (Animation & Events), Iris (Data analytics), Mia (Content & branding)
**Rédigé par** : Léo (Community Manager)

## Contexte
Le bot a déjà des briques solides (économie Coude + casino, salons vocaux temporaires, tickets, automod, progression XP, confessions, modération). L'objectif de la réunion : trouver des features **différenciantes** qui exploitent ces briques au lieu d'en empiler de nouvelles isolées.

## Décisions
- Prioriser deux features à **faible risque prod** et complémentaires :
  - **Récap hebdo agrégé** (rétention, asynchrone) : le bot raconte la semaine du serveur en embed mobile-first (5-6 items max), données agrégées.
  - **Casino provably-fair** (confiance, robuste, stateless) : commit-reveal d'un seed hashé pour que chaque spin soit vérifiable par le joueur. Rare/inexistant sur les bots grand public.
- Le **campfire vocal temps réel** (event éclair déclenché quand 5+ en vocal) ne part PAS en dev tant qu'Iris n'a pas mesuré la fréquence réelle des sessions vocales 5+.
- Règle transverse imposée à toute nouvelle feature : **embed mobile-first** (Mia) + **agrégats anonymisés avec opt-out**, aucune donnée nominative publique sans consentement (Iris, RGPD).
- Onboarding "carte d'identité de serveur" au join retenu comme chantier séparé, aligné sur la charte existante.

## Questions ouvertes
- Récap : texte templaté vs génération IA (infra ML dispo) — ton, coût, risque de dérapage.
- Provably-fair : rétro-appliqué à slot/wheel/blackjack existants, ou nouveau jeu vitrine ?
- Meta-progression "saisons / battle-pass" reliant combat + vocal + messages : à cadrer ou écarter (gros chantier).
- Campfire : puise-t-il dans la cashbox ? Impact sur l'équilibre monétaire récemment sécurisé.

## Risques identifiés
- Campfire temps réel : dépend de la détection `voice_state_update` déjà fragile (risque de double-déclenchement / events fantômes).
- Récap IA : ton hors-charte ou fuite d'info sensible (confession dé-anonymisée, sanction nominative) si prompt non cadré.
- Provably-fair : seed mal géré (révélé trop tôt / prévisible) = exploit offert, inverse de l'objectif.
- Sur-gamification : empiler récap + campfire + saisons peut noyer le nouveau et transformer le serveur en usine à points.

## Actions
- [ ] Spike data : distribution des sessions vocales par taille + funnel onboarding actuel (invite → join → 1er message → J7) — owner : Iris — échéance : avant tout dev campfire
- [ ] Prototype provably-fair (commit-reveal du seed) sur un jeu casino — owner : Kenji
- [ ] Maquette du récap hebdo (contenu agrégé, embed mobile, 5-6 items) — owner : Léo + Mia
- [ ] Règle privacy écrite (agrégats, opt-out, pas de nominatif public) applicable à toute feature — owner : Iris
- [ ] Campfire vocal en attente de la data d'Iris, avec plan B asynchrone si sessions 5+ trop rares — owner : Théo
