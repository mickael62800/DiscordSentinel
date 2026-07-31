# Illustrations de l'accueil public

Images utilisées par `PublicHomePage.vue`. Tant qu'un fichier est absent, la
section bascule automatiquement en pleine largeur — la page reste correcte,
elle n'affiche jamais d'image cassée.

## Format

- **Proportions : 4/3** (l'affichage recadre en `object-fit: cover`).
- **1200 × 900 px** suffit : elles s'affichent à ~500 px de large. Au-delà, on
  alourdit la page sans aucun gain visible.
- **JPEG progressif, qualité 82**, soit ~200 Ko par image. Les illustrations
  livrées faisaient 2,3 Mo chacune en PNG : 17 Mo pour la page, redhibitoire
  sur mobile. Le PNG est très inefficace sur des images photo-réalistes.
- Les **coins arrondis sont faits en CSS**, pas dans le fichier : plus net,
  adaptatif, et pas de canal alpha à payer.

Script d'optimisation employé (à rejouer après chaque livraison) : redimension
en 1200×900 puis export JPEG progressif qualité 82.

## Direction artistique

Reprendre l'univers du logo pour que l'ensemble se tienne :

- dominante **violet néon sur fond sombre** ;
- ambiance **chaleureuse et cosy**, pas corporate ;
- le **pingouin de la bande** peut apparaître, c'est la mascotte ;
- **pas de texte incrusté** dans l'image : il est déjà dans la page, et un
  texte gravé ne serait ni modifiable ni lisible sur petit écran.

## Fichiers utilisés par l'accueil

| Fichier | Section | Contenu |
|---|---|---|
| `section-jeux.jpg` | Nos serveurs | Le pingouin devant ses écrans de jeu, rack de serveurs violet en fond. |
| `section-vocal.jpg` | La vie du serveur | Plusieurs pingouins au casque sur le canapé, ambiance soirée. |
| `section-planning.jpg` | Le planning | Calendrier stylisé, campagnes qui s'étalent sur plusieurs semaines. |
| `section-animation.jpg` | Concours | Tirage au sort, roue de tombola, boîtes-cadeaux. **Pas de podium** : il appartient aux classements. |
| `section-classements.jpg` | Classements | Podium, coupe, barres de progression. |
| `section-moderation.jpg` | Un cadre sain | Le pingouin en gardien tranquille — rassurant, jamais policier. |

## Fichiers en réserve

Livrés mais pas encore employés, en attendant leurs pages dédiées :

| Fichier | Destination prévue |
|---|---|
| `section-annonces.jpg` | Page des annonces, alimentée par le module du bot. |
| `section-galerie.jpg` | Galerie de captures d'écran de la communauté. |

## Ajouter une section

Les sections sont déclarées dans la constante `SECTIONS` de
`web/src/components/pages/PublicHomePage.vue`. Ajouter une entrée suffit :
l'alternance gauche/droite et l'animation d'apparition sont automatiques.
