import { describe, expect, it } from "vitest";
import { readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

import { BANNIERES, banniereDepuisUrl, categories } from "./bannieres";

const ICI = dirname(fileURLToPath(import.meta.url));
const DOSSIER_IMAGES = resolve(ICI, "../../public/imgs");

function fichiersSurDisque(): string[] {
  return readdirSync(DOSSIER_IMAGES).filter((f) => /\.(jpe?g|png|webp|gif)$/i.test(f));
}

describe("catalogue des bannières", () => {
  // Le catalogue est écrit à la main : sans ces deux tests il dérive
  // silencieusement du dossier, et la dérive ne se voit qu'à l'usage —
  // une image proposée qui ne s'affiche pas, ou une image livrée que
  // personne ne peut choisir.

  it("ne référence que des fichiers réellement présents", () => {
    const surDisque = new Set(fichiersSurDisque());
    const manquants = BANNIERES.filter((b) => !surDisque.has(b.fichier)).map((b) => b.fichier);
    expect(manquants, "images cataloguées mais absentes du dossier").toEqual([]);
  });

  it("couvre toutes les images du dossier", () => {
    const catalogues = new Set(BANNIERES.map((b) => b.fichier));
    const oubliees = fichiersSurDisque().filter((f) => !catalogues.has(f));
    expect(oubliees, "images présentes mais absentes du catalogue").toEqual([]);
  });

  it("n'a ni fichier ni libellé en double", () => {
    // Deux entrées pour le même fichier donneraient deux lignes identiques
    // dans le sélecteur ; deux libellés identiques rendraient le choix ambigu.
    const fichiers = BANNIERES.map((b) => b.fichier);
    const libelles = BANNIERES.map((b) => b.libelle);
    expect(new Set(fichiers).size).toBe(fichiers.length);
    expect(new Set(libelles).size).toBe(libelles.length);
  });

  it("classe chaque image dans une catégorie renseignée", () => {
    expect(BANNIERES.every((b) => b.categorie.trim().length > 0)).toBe(true);
    expect(categories().length).toBeGreaterThan(0);
  });
});

describe("banniereDepuisUrl", () => {
  it("retrouve l'image quel que soit le domaine", () => {
    // L'URL stockée porte le domaine du jour où elle a été choisie. En
    // changer ne doit pas faire « oublier » au sélecteur l'image retenue.
    const attendu = BANNIERES[0].fichier;
    for (const url of [
      `https://exemple.fr/imgs/${attendu}`,
      `http://autre-domaine.net/imgs/${attendu}`,
      `/imgs/${attendu}`,
    ]) {
      expect(banniereDepuisUrl(url)?.fichier).toBe(attendu);
    }
  });

  it("rend null pour une URL externe ou vide", () => {
    // Le sélecteur bascule alors en saisie libre plutôt que d'afficher un
    // choix vide qui donnerait l'impression d'avoir perdu la valeur.
    expect(banniereDepuisUrl("https://cdn.exemple.fr/photo.png")).toBeNull();
    expect(banniereDepuisUrl("")).toBeNull();
    expect(banniereDepuisUrl(null)).toBeNull();
  });
});
