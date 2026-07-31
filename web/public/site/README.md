# Illustrations de l'accueil public

Images attendues par `PublicHomePage.vue`. Tant qu'un fichier est absent, la
section bascule automatiquement en pleine largeur — la page reste correcte,
elle n'affiche jamais d'image cassée.

## Format commun

- **Proportions : 4/3** (l'affichage recadre en `object-fit: cover`).
- **Taille : 1200 × 900 px** suffit largement. Au-delà, on alourdit la page
  sans gain visible : elles s'affichent à ~500 px de large.
- **Livrer en PNG ou JPG** : le script d optimisation convertit en JPEG progressif 1200x900 (~200 Ko).
- **PNG ou JPG.** JPG de préférence pour les scènes photo-réalistes, il pèse
  bien moins lourd. PNG si l'image contient des aplats ou du texte net.
- **Poids visé : moins de 300 Ko chacune.** Les logos livrés faisaient 2 Mo
  pour un affichage en 300 px ; ici elles se chargent en différé
  (`loading="lazy"`) mais restent visibles dès le premier défilement.

## Direction artistique

Reprendre l'univers du logo pour que l'ensemble se tienne :

- dominante **violet néon sur fond sombre** ;
- ambiance **chaleureuse et cosy**, pas corporate ;
- le **pingouin de la bande** peut apparaître, c'est la mascotte ;
- pas de texte incrusté dans l'image : il est déjà dans la page, et un texte
  gravé ne serait ni traduisible ni lisible sur petit écran.

## Fichiers (livres, optimises en JPEG)

| Fichier | Section | Contenu suggéré |
|---|---|---|
| `section-jeux.png` | Nos serveurs | Le pingouin devant plusieurs écrans de jeu, ou un établi / rack de serveurs stylisé aux couleurs violettes. Évoque Minecraft et Palworld sans copier leurs visuels. |
| `section-vocal.png` | La vie du serveur | Plusieurs pingouins sur le canapé avec des casques audio, bulles de discussion, ambiance soirée. |
| `section-animation.png` | Concours et classements | Podium, coupe, confettis, pièces de la monnaie du serveur. Ton joueur et un peu taquin. |
| `section-moderation.png` | Un cadre sain | Le pingouin en gardien tranquille : bouclier doux, veilleuse, ambiance rassurante — surtout pas policière ni menaçante. |

## Ajouter une section

Les sections sont déclarées dans la constante `SECTIONS` de
`web/src/components/pages/PublicHomePage.vue`. Ajouter une entrée suffit :
l'alternance gauche/droite et l'animation d'apparition sont automatiques.
