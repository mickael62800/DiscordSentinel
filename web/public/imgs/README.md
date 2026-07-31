# Images partagées — bot Discord et site web

**Un seul fichier, deux usages.** Tout ce qui est déposé ici est servi par
nginx à la racine du site, donc accessible aux deux :

```
web/public/imgs/bienvenue.png
   → site    : /imgs/bienvenue.png
   → Discord : https://<WEB_DOMAIN>/imgs/bienvenue.png
```

## Pourquoi ici et pas dans le bot

Discord **ne peut pas lire un fichier local** du conteneur du bot. Un embed
référence une **URL publique en HTTPS** que Discord va chercher lui-même sur
notre serveur. C'est donc nginx qui doit publier ces images — d'où leur place
dans `web/public/`.

Le bot construit l'URL à partir de `WEB_FRONT_URL`.

## Trois règles à respecter

### 1. Ne jamais réécrire une image sous le même nom

Discord met les images en cache **par URL**, de façon très agressive. Remplacer
`bienvenue.png` par une nouvelle version en gardant le nom laisse Discord
afficher l'ancienne pendant des jours.

Changer de nom à chaque révision : `bienvenue-2.png`, `bienvenue-3.png`.

### 2. Optimiser AVANT de committer

Viser **1000 px de large et moins de 300 Ko**. Discord recompresse de toute
façon derrière, et une image lourde ralentit à la fois le bot qui la poste et
le visiteur qui charge la page.

L'historique git est **définitif** : une image de 2 Mo commitée par erreur
pèsera dans le dépôt pour toujours, même supprimée ensuite.

### 3. Ces images doivent être commitées

Sans ça, un `git pull` sur le serveur ne les apporte pas, et ni le site ni le
bot ne les trouvent. C'est justement pourquoi la règle 2 compte.

## Organisation

| Dossier | Usage |
|---|---|
| `imgs/` | Images **partagées** : annonces, bienvenue, départs, classements, promotions — tout ce que les bots postent et que le site peut réafficher. |
| `site/` | Illustrations de la vitrine publique uniquement (sections de l'accueil). |
| racine de `public/` | Logos et favicon : identité, utilisée partout. |

Si `imgs/` grossit beaucoup, le découper par domaine (`imgs/annonces/`,
`imgs/classements/`) plutôt que par bot : une même image sert souvent à
plusieurs modules.
