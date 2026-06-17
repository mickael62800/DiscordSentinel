# 🐾 Tamagotchi — Prompts de génération des évolutions

Objectif : générer les sprites d'avatar pour chaque animal, à chaque **stade**
d'évolution et chaque **état** visuel, via ChatGPT/DALL·E.

## Récap

- **6 animaux** : `sanglier`, `renard`, `tortue`, `loup`, `lapin`, `ours`
- **4 stades** (par niveau) :
  | Stade | Niveaux |
  |---|---|
  | `bebe` | 1 – 4 |
  | `jeune` | 5 – 14 |
  | `adulte` | 15 – 29 |
  | `vieux` | 30 + |
- **4 états** (par jauges/santé) : `content`, `affame`, `fatigue`, `malade`
- **1 logo partagé** : `mort.png` (overlay/avatar quand le compagnon est mort)

**Total : 6 × 4 × 4 = 96 images + `mort.png` = 97 fichiers.**

## Nommage des fichiers (IMPORTANT — le code attend exactement ça)

```
{espece}_{stade}_{etat}.png
```
minuscules, sans accents. Ex. `loup_adulte_content.png`, `renard_bebe_malade.png`.
Le mort : `mort.png`.

## Specs

- **PNG, fond transparent**, **512×512**, format carré.
- Sujet **centré**, **corps entier**, vue de face, **même cadrage/échelle** partout.
- ChatGPT ne fait pas de transparence fiable → génère avec **fond uni** puis détoure.

---

## Style maître (à coller AVANT chaque image)

```
Mascotte d'animal style jeu mobile mignon (kawaii / chibi) : grosse tête, grands
yeux expressifs, contour épais arrondi, ombrage cel doux, couleurs vives et
saturées. Vue de face, corps entier, sujet parfaitement centré, même échelle et
même cadrage à chaque image. Fond UNI d'une seule couleur plate (pastel) pour
détourage facile. Aucun texte, aucune bordure. Format carré 1:1.
```

## Descripteurs de stade (communs à tous)

- `bebe` : tout petit, tête surdimensionnée, yeux énormes brillants, maladroit
- `jeune` : adolescent svelte, posture énergique
- `adulte` : adulte, stature pleine et assurée
- `vieux` : poil grisonnant/blanchi, petites lunettes rondes, rides légères, air calme et sage (canne optionnelle)

## Descripteurs d'état (communs à tous)

- `content` : grand sourire, yeux pétillants, pose joyeuse
- `affame` : yeux suppliants, patte sur le ventre, bulle de pensée avec de la nourriture
- `fatigue` : yeux mi-clos, bâillement, « Zzz », oreilles tombantes
- `malade` : teint verdâtre, thermomètre dans la bouche, goutte de sueur, air patraque

## Descripteurs d'espèce

| Espèce | Description |
|---|---|
| `sanglier` | sanglier brun, petites défenses, poil hérissé |
| `renard` | renard orange, ventre blanc, grande queue touffue, yeux malicieux |
| `tortue` | tortue verte avec carapace ronde |
| `loup` | loup gris au pelage fourni, poitrail clair, yeux ambrés |
| `lapin` | lapin aux longues oreilles, museau rose |
| `ours` | ours brun rond et dodu |

---

## Méthode recommandée (cohérence du personnage)

ChatGPT ne garde pas un perso identique entre 2 prompts séparés. Pour **chaque
animal**, dans **une seule conversation** :

1. Génère d'abord l'image de **référence** = `{espece}_adulte_content` (avec le
   style maître + descripteur d'espèce).
2. Puis demande les **15 autres** : « *Même {espèce}, même style et même cadrage,
   mais : {stade} + {état}* ».
3. Détoure chaque image (fond uni → PNG transparent 512×512) et nomme-la.

### En-tête prêt à coller (remplace `{ESPECE}` + sa description)

```
Génère une mascotte de jeu mobile, style mignon kawaii/chibi : grosse tête,
grands yeux expressifs, contour épais arrondi, ombrage cel doux, couleurs vives.
Personnage = {DESCRIPTION ESPECE}. Vue de face, corps entier, centré, fond uni
pastel d'une seule couleur (pour détourage), aucun texte, format carré 1:1.
Première image de référence : {ESPECE} ADULTE, état CONTENT (grand sourire, yeux
pétillants, pose fière).
```
→ `{espece}_adulte_content.png`

### Les 16 combinaisons par animal

```
{espece}_bebe_content     {espece}_jeune_content     {espece}_adulte_content     {espece}_vieux_content
{espece}_bebe_affame      {espece}_jeune_affame      {espece}_adulte_affame      {espece}_vieux_affame
{espece}_bebe_fatigue     {espece}_jeune_fatigue     {espece}_adulte_fatigue     {espece}_vieux_fatigue
{espece}_bebe_malade      {espece}_jeune_malade      {espece}_adulte_malade      {espece}_vieux_malade
```

---

## Logo mort (1 seul, partagé)

```
Icône simple et lisible : petite pierre tombale grise stylisée (ou auréole +
petite croix mignonne), style flat, contour épais, fond uni/transparent, 1:1.
Sobre, pas effrayant.
```
→ `mort.png`

---

## Intégration (côté code)

Les images sont chargées depuis le dossier défini par la variable d'env
**`TAMAGOTCHI_SPRITES_DIR`** (ex. `/assets/tamagotchi`). Le rendu choisit
automatiquement le fichier selon `(espèce, niveau→stade, jauges/santé→état)`.
Si un fichier manque (ou la variable n'est pas définie), on retombe sur l'ancien
placeholder (cercle coloré + initiale) — rien ne casse.
