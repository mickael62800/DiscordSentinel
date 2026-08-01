// Disposition des événements dans une vue semaine.
//
// Le calendrier est une grille CSS de 7 colonnes. Chaque événement devient une
// barre qui occupe `span` colonnes à partir de `from`. Tout l'enjeu est de
// placer les barres sur des lignes distinctes quand elles se chevauchent —
// sinon deux campagnes simultanées se superposeraient et l'une masquerait
// l'autre.

import type { PublicEvent } from "@/services/publicEventsService";

/** Une barre prête à être posée dans la grille. */
export interface WeekBar {
  event: PublicEvent;
  /** Colonne de départ, 1 à 7 (lundi = 1). */
  from: number;
  /** Nombre de colonnes occupées, 1 à 7. */
  span: number;
  /** Ligne de la grille, à partir de 1. */
  row: number;
  /** Vrai si l'événement a commencé avant le lundi affiché. */
  clippedStart: boolean;
  /** Vrai s'il se termine après le dimanche affiché. */
  clippedEnd: boolean;
}

const JOUR_MS = 24 * 60 * 60 * 1000;

/** Minuit local du jour de `d`. Sert de base à tous les calculs de jour. */
export function startOfDay(d: Date): Date {
  const c = new Date(d);
  c.setHours(0, 0, 0, 0);
  return c;
}

/**
 * Lundi de la semaine contenant `d`.
 *
 * `getDay()` renvoie 0 pour dimanche : sans le décalage, la semaine
 * commencerait le dimanche, ce qui n'est pas la convention française.
 */
export function startOfWeek(d: Date): Date {
  const base = startOfDay(d);
  const jour = base.getDay();
  const recul = jour === 0 ? 6 : jour - 1;
  base.setDate(base.getDate() - recul);
  return base;
}

/** Décale d'un nombre de semaines, positif ou négatif. */
export function addWeeks(d: Date, n: number): Date {
  const c = new Date(d);
  c.setDate(c.getDate() + n * 7);
  return c;
}

/** Les 7 jours de la semaine commençant à `weekStart`. */
export function weekDays(weekStart: Date): Date[] {
  return Array.from({ length: 7 }, (_, i) => {
    const d = new Date(weekStart);
    d.setDate(d.getDate() + i);
    return d;
  });
}

/** Nombre de jours entiers entre deux minuits. */
function diffDays(from: Date, to: Date): number {
  return Math.round((startOfDay(to).getTime() - startOfDay(from).getTime()) / JOUR_MS);
}

/**
 * Place les événements qui chevauchent la semaine.
 *
 * Les événements sont triés par date de début puis par durée décroissante :
 * les campagnes longues se posent en premier, sur les lignes du haut, et les
 * soirées ponctuelles se glissent en dessous. L'inverse donnerait un
 * calendrier en escalier, illisible.
 */
export function layoutWeek(events: PublicEvent[], weekStart: Date): WeekBar[] {
  const weekEnd = new Date(weekStart);
  weekEnd.setDate(weekEnd.getDate() + 7);

  const candidats = events
    .map((event) => ({ event, debut: new Date(event.starts_at), fin: new Date(event.ends_at) }))
    // Chevauchement, pas date de début : une campagne commencée la semaine
    // dernière doit apparaître dans celle-ci.
    .filter(({ debut, fin }) => debut < weekEnd && fin >= weekStart)
    .sort((a, b) => {
      const parDebut = a.debut.getTime() - b.debut.getTime();
      if (parDebut !== 0) return parDebut;
      return b.fin.getTime() - b.debut.getTime() - (a.fin.getTime() - a.debut.getTime());
    });

  // Pour chaque ligne, la dernière colonne occupée. Une barre se pose sur la
  // première ligne dont la place est libre.
  const finDeLigne: number[] = [];
  const barres: WeekBar[] = [];

  for (const { event, debut, fin } of candidats) {
    const from = Math.max(1, diffDays(weekStart, debut) + 1);
    const to = Math.min(7, diffDays(weekStart, fin) + 1);
    const span = Math.max(1, to - from + 1);

    let row = finDeLigne.findIndex((occupe) => occupe < from);
    if (row === -1) {
      row = finDeLigne.length;
      finDeLigne.push(0);
    }
    finDeLigne[row] = from + span - 1;

    barres.push({
      event,
      from,
      span,
      row: row + 1,
      clippedStart: debut < weekStart,
      clippedEnd: fin >= weekEnd,
    });
  }

  return barres;
}

/** Libellé de la semaine, par exemple « 3 – 9 février ». */
export function weekLabel(weekStart: Date): string {
  const fin = new Date(weekStart);
  fin.setDate(fin.getDate() + 6);

  const jour = (d: Date) => d.toLocaleDateString("fr-FR", { day: "numeric" });
  const mois = (d: Date) => d.toLocaleDateString("fr-FR", { month: "long" });

  // Une semaine à cheval sur deux mois doit les nommer tous les deux, sinon
  // « 30 – 5 février » laisse croire que le 30 est en février.
  if (weekStart.getMonth() === fin.getMonth()) {
    return `${jour(weekStart)} – ${jour(fin)} ${mois(fin)}`;
  }
  return `${jour(weekStart)} ${mois(weekStart)} – ${jour(fin)} ${mois(fin)}`;
}
