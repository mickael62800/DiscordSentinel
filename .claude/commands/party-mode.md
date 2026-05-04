---
description: Réunion multi-personas autour d'un sujet (style BMAD party-mode)
argument-hint: <sujet>
---

# /party-mode

Tu animes une **réunion d'équipe** entre personas du dossier `persona/` (à la racine du projet) autour du sujet donné en argument.

## Étape 1 — Choisir le casting

Lis d'abord `persona/README.md` pour la liste des personas disponibles dans ce projet.

- Sélectionne **3 à 6 personas pertinents** selon le sujet (au-delà la réunion devient illisible).
- Si peu de personas existent (≤ 6), tu peux tous les inclure.
- Si le sujet ne concerne qu'une partie de l'équipe, ne convoque pas les autres pour rien.

Lis ensuite intégralement les fichiers `.md` des personas retenus pour t'imprégner de leur ton, obsessions, rejets et bonnes pratiques.

## Étape 2 — Annoncer le casting

```
**Casting** : Léa (Architect), Marc (Backend), Rachid (Security), Hugo (DevOps)
**Sujet** : <sujet>
```

## Étape 3 — Dérouler la réunion en 4 phases

### Phase 1 — Tour de table
Chaque persona prend la parole une fois, dans l'ordre, avec sa **voix propre**. 1 paragraphe court (3-6 lignes).

Format :
```
**[Léa — Architect]** ...
**[Marc — Backend]** ...
```

### Phase 2 — Débat
**Au moins 2 désaccords explicites** entre personas sur des points concrets. Pas de consensus mou. Format dialogué :
```
**[Rachid]** ...
**[Hugo]** Désaccord — ...
```

### Phase 3 — Synthèse
- **Décisions actées** : liste à puces.
- **Questions ouvertes** : liste à puces.

### Phase 4 — Risques & next steps
- **Ce qui peut casser** : 2-4 risques concrets.
- **Next steps** : actions avec owner persona (ex: "Marc spike OAuth flow").

### Phase 5 — Compte-rendu (CR)
Le **chef de projet** présent dans le casting (Clara pour IT, Julien pour VRD, Léo pour Discord, Raphaël pour trading, ou équivalent) rédige le **CR de la réunion**.

Format du CR :
```
# CR — <sujet> — <date>
**Présents** : <liste des personas>
**Rédigé par** : <nom du chef de projet>

## Décisions
- ...

## Questions ouvertes
- ...

## Risques identifiés
- ...

## Actions
- [ ] <action> — owner : <persona> — échéance : <si pertinente>
```

Si aucun "chef de projet" n'est dans le casting, c'est le persona le plus senior / transverse qui rédige (ex: l'architect).

Sauvegarder le CR dans `cr/<YYYY-MM-DD>_<slug-sujet>.md` à la racine du projet (créer le dossier `cr/` si absent).

## Règles internes

- Ne pas inventer de personas absents du dossier.
- Respecter le **pragmatisme** des personas (pas de dogmatisme).
- Interventions **brèves et tranchantes**, pas de remplissage.
- Si un persona n'a rien de pertinent : il passe son tour explicitement.
- Pas d'emoji.

## Sujet

$ARGUMENTS
