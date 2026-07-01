# Gestion intelligente du serveur — pistes d'évolution

> Document d'idées (product brief léger). Objectif : rendre la **gestion** du serveur
> (modération, sécurité, santé de la commu, charge du staff) plus **intelligente,
> proactive et cohérente** — en réutilisant les briques déjà en place plutôt qu'en
> empilant des features isolées.
>
> Date : 2026-07-01 · Statut : idées à arbitrer, rien d'engagé.

## Principe directeur

Le bot a déjà des briques fortes (automod IA texte/vision, strikes + escalade,
`channel_tension` / `evaluate_flood`, tickets, watched users, RBAC, progression).
La plupart sont **réactives et isolées**. L'idée transverse : ajouter une couche
**conseil / prédiction / adaptation** au-dessus, pour passer d'un bot qui *exécute*
à un bot qui *aide à décider*.

## Règle transverse (non négociable)

Toute feature ci-dessous manipulant du comportement membre doit être :
- **Agrégée et privée** : signaux visibles du staff uniquement, jamais affichés publiquement.
- **Opt-out** possible, conforme RGPD (pas de profilage nominatif public, droit à l'effacement).
- **Explicable** : toute décision automatique doit pouvoir dire *pourquoi* (quelle règle, quel score).

---

## 1. Copilote de modération  ⭐ priorité haute

**Problème** : chaque modo sanctionne à sa manière → incohérence, injustice ressentie,
décisions au feeling.

**Ce qu'on fait** : au moment où un modo agit (ou via une commande `/contexte @membre`),
le bot affiche l'historique complet + une **suggestion proportionnée basée sur les
précédents** : « ce membre a 2 strikes actifs ; des cas similaires ont pris un mute 24h ».

**Pourquoi c'est différenciant** : de la **modération jurisprudentielle** — cohérence
entre modos. Quasi personne ne le fait.

**Briques réutilisées** : strikes + escalade (déjà fiabilisés), historique d'actions,
cards de log de sanction.

**Complexité / risque** : moyenne, faible risque prod (surface = affichage + reco, pas
d'action auto). Le "moteur de précédents" peut démarrer simple (règles sur le compte de
strikes) puis s'affiner.

## 2. Anti-raid prédictif  ⭐ priorité haute (sécurité)

**Problème** : l'automod réagit *au message*, donc après les premiers dégâts d'un raid.

**Ce qu'on fait** : scorer les **patterns de join en temps réel** — vélocité des arrivées,
âge des comptes en grappe, pseudos similaires, absence d'avatar/bannière — et **proposer
un lockdown / une quarantaine AVANT** l'attaque, avec confirmation staff.

**Pourquoi c'est différenciant** : passer de réactif à **prédictif**.

**Briques réutilisées** : `watched_users`, infra d'events, quarantaine/lockdown (sécurité-hôte).

**Complexité / risque** : moyenne. **Risque clé = faux positifs** (lockdown injustifié) →
démarrer en mode *suggestion*, seuils prudents, jamais 100% auto au début.

## 3. Automod adaptatif au ton du salon

**Problème** : seuils statiques → un même mot est traité pareil dans #memes et #annonces.

**Ce qu'on fait** : apprendre la **baseline normale de chaque salon** et flaguer les
**écarts** au lieu d'un seuil fixe.

**Pourquoi c'est différenciant** : un automod qui s'adapte au **contexte** plutôt que
d'imposer une règle unique.

**Briques réutilisées** : `channel_tension` / `evaluate_flood` (déjà là), scoring automod.

**Complexité / risque** : moyenne-haute (calibration, risque de faux positifs si la
baseline est mal apprise). À faire **après** #1/#2.

## 4. Score de confiance transverse (friction adaptative)

**Problème** : la modération est binaire (sanctionné / pas sanctionné), sans mémoire du
"bon comportement".

**Ce qu'on fait** : agréger le comportement (ancienneté, messages utiles, sanctions,
tickets résolus) en un **signal de confiance privé** qui **module la friction** : un
vétéran passe sous le radar automod, un compte neuf / low-trust est surveillé plus finement.

**Pourquoi c'est différenciant** : friction **adaptative** au lieu d'uniforme.

**Briques réutilisées** : progression, strikes, tickets, automod.

**Complexité / risque** : haute (définir le score sans biais, éviter le "système de note
sociale"). **Strictement privé**, jamais exposé au membre. À cadrer sérieusement.

## 5. Triage intelligent des tickets

**Problème** : les tickets arrivent en vrac, le staff trie et répond à la main.

**Ce qu'on fait** : classer chaque ticket entrant (bug / signalement / question) via IA,
**prioriser, détecter les doublons, suggérer une réponse type**.

**Pourquoi c'est différenciant** : le staff traite nettement plus vite, sans perte
d'information.

**Briques réutilisées** : module tickets (déjà durci), infra ML.

**Complexité / risque** : moyenne, faible risque (assistance, pas d'action destructive).

## 6. Rapport de santé / hygiène hebdo (staff-side)

**Problème** : la dérive du serveur est invisible jusqu'à ce qu'elle fasse mal (salons
morts, rôles sur-permissionnés, membres qui décrochent).

**Ce qu'on fait** : un rapport hebdo **destiné au staff** : salons morts à archiver, rôles
sur-permissionnés, **membre actif qui décroche** (à ré-engager), **contributeur montant**
(à promouvoir régular).

**Pourquoi c'est différenciant** : de la data agrégée → **des décisions**, pas des vanity metrics.

**Briques réutilisées** : analytics, progression, RBAC, reconciler (concept déjà présent côté voice).

**Complexité / risque** : moyenne, faible risque (rapport lecture seule).

---

## Priorisation proposée

| Ordre | Idée | Valeur | Risque | Réutilise l'existant |
|---|---|---|---|---|
| 1 | Copilote de modération (#1) | Haute | Faible | ++ |
| 2 | Anti-raid prédictif (#2) | Haute | Moyen (faux positifs) | + |
| 3 | Triage tickets (#5) | Moyenne-haute | Faible | ++ |
| 4 | Rapport de santé hebdo (#6) | Moyenne | Faible | + |
| 5 | Automod adaptatif (#3) | Moyenne-haute | Moyen-haut | ++ |
| 6 | Score de confiance (#4) | Haute | Haut (à cadrer) | + |

**Recommandation** : commencer par le **copilote de modération (#1)** — plus haut ratio
valeur/risque pour un serveur perso avec des modos, sur des briques déjà fiabilisées.

## Points d'attention communs

- **Faux positifs** : toute automatisation (raid, automod adaptatif, confiance) démarre en
  mode *suggestion au staff*, jamais 100 % auto au lancement.
- **Explicabilité** : chaque signal/score doit afficher son "pourquoi".
- **Privacy** : agrégats privés, opt-out, aucune donnée nominative publique.
- **Charge staff** : l'objectif est de **réduire** la charge, pas d'ajouter des dashboards
  que personne ne lit — un seul rapport utile > cinq orphelins.

## Prochaine étape suggérée

Choisir 1 idée à spécifier finement (schéma DB, config par serveur, surface bot/web) —
le copilote de modération est le candidat recommandé pour un premier incrément.
