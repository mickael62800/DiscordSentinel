# CR — Automod dans l'usage — 2026-07-02
**Présents** : Léo (Community Manager), Ava (Trust & Safety), Nora (Mod Lead), Kenji (Admin technique / bot dev), Iris (Data analytics)
**Rédigé par** : Léo (Community Manager)

## Contexte
Revue de l'usage du module automod maison de DiscordSentinel : scoring IA texte/vision, mute auto, votes de review par les modos, mode nuit, configuration par serveur. Objectif : cadrer jusqu'où l'automatisation décide seule, et sous quelles garanties.

## Décisions
- Cascade assumée : AutoMod natif Discord en première ligne (patterns évidents), scoring IA maison en complément là où le natif ne sait pas faire (vision, contexte, phishing déguisé). Le natif doit tenir seul si l'IA tombe.
- Le scoring IA généraliste reste en warn / timeout court — pas de mute ou ban autonome — tant que le taux de faux positifs n'est pas mesuré.
- Durcissement des comptes de moins de 7 jours ciblé sur liens + mentions, pas sur le score global (ne pas punir la nouveauté).
- Toute action automatique notifie au membre le motif + la voie d'appel (conformité DSA, cohérence de ton).
- Réversibilité garantie : les logs doivent permettre d'annuler un faux positif de masse.
- Graceful degradation : en cas de panne d'inférence, l'automod continue en mode règles simples.

## Questions ouvertes
- Taux de faux positifs actuel, par type de flag et par cohorte de comptes : non mesuré aujourd'hui.
- Les seuils modifiés dans le dashboard ne s'appliquent qu'au redémarrage du bot (trackers figés au démarrage) : recharge à chaud, ou avertissement explicite dans l'UI ?
- Mode nuit : sur quels signaux exactement le durcissement s'applique, et validé par quelles données ?
- Mute auto court : durée par défaut, et notification avec compte à rebours au membre ?

## Risques identifiés
- Sur-blocage silencieux : membres légitimes découragés, invisibles dans les logs de sanction, érosion de la rétention J7.
- Config fantôme : un seuil changé mais ignoré jusqu'au reboot → décisions sur un réglage inactif.
- Point de défaillance IA : pic de charge ou panne d'inférence pendant un raid.
- Non-conformité DSA : action auto sans motif ni appel = risque légal UE, au-delà de l'UX.

## Actions
- [ ] Instrumenter le ratio actions-auto / annulées-par-modo (par flag et cohorte) et produire une baseline avant tout durcissement — owner : Iris
- [ ] Trancher le rechargement à chaud des seuils, ou afficher un avertissement "actif au prochain redémarrage" ; vérifier le fallback règles-simples si l'IA tombe — owner : Kenji
- [ ] Définir la matrice sanction par flag (warn / timeout / jamais auto) + gabarit de message d'appel — owner : Nora
- [ ] Valider le ton et le contenu des messages d'action auto envoyés au membre — owner : Léo
- [ ] Spécifier le durcissement ciblé comptes < 7 jours (liens/mentions) + procédure de rollback d'un faux positif de masse — owner : Ava
