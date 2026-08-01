import { describe, expect, it } from "vitest";

import {
  addWeeks,
  layoutWeek,
  startOfWeek,
  weekDays,
  weekLabel,
} from "./useWeekPlanning";
import type { PublicEvent } from "@/services/publicEventsService";

/** Événement minimal : seules les bornes comptent pour la disposition. */
function ev(id: string, debut: string, fin: string): PublicEvent {
  return {
    id,
    title: id,
    description: null,
    game: null,
    color: null,
    starts_at: new Date(debut).toISOString(),
    ends_at: new Date(fin).toISOString(),
    all_day: false,
    span_days: 1,
  };
}

// Lundi 2 février 2026.
const LUNDI = new Date(2026, 1, 2);

describe("startOfWeek", () => {
  it("ramène un jour de semaine au lundi", () => {
    expect(startOfWeek(new Date(2026, 1, 5)).getDate()).toBe(2);
  });

  it("est stable si on est déjà lundi", () => {
    expect(startOfWeek(LUNDI).getDate()).toBe(2);
  });

  // getDay() renvoie 0 le dimanche : sans décalage, la semaine commencerait
  // ce jour-là.
  it("rattache le dimanche à la semaine qui vient de s'écouler", () => {
    expect(startOfWeek(new Date(2026, 1, 8)).getDate()).toBe(2);
  });

  it("remet l'heure à minuit", () => {
    const d = startOfWeek(new Date(2026, 1, 5, 23, 47));
    expect([d.getHours(), d.getMinutes(), d.getSeconds()]).toEqual([0, 0, 0]);
  });
});

describe("weekDays", () => {
  it("donne sept jours consécutifs", () => {
    const jours = weekDays(LUNDI);
    expect(jours).toHaveLength(7);
    expect(jours.map((d) => d.getDate())).toEqual([2, 3, 4, 5, 6, 7, 8]);
  });
});

describe("addWeeks", () => {
  it("avance et recule d'une semaine", () => {
    expect(addWeeks(LUNDI, 1).getDate()).toBe(9);
    expect(addWeeks(LUNDI, -1).getDate()).toBe(26);
  });
});

describe("layoutWeek", () => {
  it("place un événement d'un jour sur sa colonne", () => {
    const [bar] = layoutWeek([ev("a", "2026-02-04T21:00", "2026-02-04T23:00")], LUNDI);
    expect(bar.from).toBe(3);
    expect(bar.span).toBe(1);
    expect(bar.row).toBe(1);
  });

  it("étale un événement de plusieurs jours", () => {
    const [bar] = layoutWeek([ev("a", "2026-02-06T10:00", "2026-02-08T18:00")], LUNDI);
    expect(bar.from).toBe(5);
    expect(bar.span).toBe(3);
  });

  // Le point du modèle par plage : une campagne de trois semaines doit
  // apparaître dans chacune des semaines qu'elle couvre.
  it("affiche une campagne commencée avant la semaine", () => {
    const [bar] = layoutWeek([ev("a", "2026-01-20T00:00", "2026-02-20T00:00")], LUNDI);
    expect(bar.from).toBe(1);
    expect(bar.span).toBe(7);
    expect(bar.clippedStart).toBe(true);
    expect(bar.clippedEnd).toBe(true);
  });

  it("marque seulement le bord réellement tronqué", () => {
    const [bar] = layoutWeek([ev("a", "2026-02-04T00:00", "2026-02-20T00:00")], LUNDI);
    expect(bar.clippedStart).toBe(false);
    expect(bar.clippedEnd).toBe(true);
  });

  it("ignore ce qui ne chevauche pas la semaine", () => {
    const bars = layoutWeek(
      [
        ev("avant", "2026-01-20T00:00", "2026-01-25T00:00"),
        ev("apres", "2026-02-10T00:00", "2026-02-12T00:00"),
      ],
      LUNDI,
    );
    expect(bars).toHaveLength(0);
  });

  // Sans lignes distinctes, deux campagnes simultanées se superposeraient et
  // l'une masquerait l'autre.
  it("sépare sur deux lignes deux événements qui se chevauchent", () => {
    const bars = layoutWeek(
      [
        ev("a", "2026-02-02T00:00", "2026-02-06T00:00"),
        ev("b", "2026-02-04T00:00", "2026-02-08T00:00"),
      ],
      LUNDI,
    );
    expect(bars.map((b) => b.row)).toEqual([1, 2]);
  });

  it("réutilise une ligne quand la place s'est libérée", () => {
    const bars = layoutWeek(
      [
        ev("a", "2026-02-02T00:00", "2026-02-03T00:00"),
        ev("b", "2026-02-05T00:00", "2026-02-06T00:00"),
      ],
      LUNDI,
    );
    expect(bars.map((b) => b.row)).toEqual([1, 1]);
  });

  // Les campagnes longues en haut, les soirées en dessous : l'inverse
  // donnerait un calendrier en escalier.
  it("pose la campagne la plus longue au-dessus à date de début égale", () => {
    const bars = layoutWeek(
      [
        ev("courte", "2026-02-02T00:00", "2026-02-02T23:00"),
        ev("longue", "2026-02-02T00:00", "2026-02-07T00:00"),
      ],
      LUNDI,
    );
    const longue = bars.find((b) => b.event.id === "longue");
    expect(longue?.row).toBe(1);
  });

  it("empile trois événements simultanés sur trois lignes", () => {
    const bars = layoutWeek(
      [
        ev("a", "2026-02-03T00:00", "2026-02-06T00:00"),
        ev("b", "2026-02-03T00:00", "2026-02-06T00:00"),
        ev("c", "2026-02-03T00:00", "2026-02-06T00:00"),
      ],
      LUNDI,
    );
    expect(new Set(bars.map((b) => b.row)).size).toBe(3);
  });

  it("borne toujours la barre dans la grille de sept colonnes", () => {
    const bars = layoutWeek(
      [ev("a", "2025-12-01T00:00", "2027-01-01T00:00")],
      LUNDI,
    );
    for (const b of bars) {
      expect(b.from).toBeGreaterThanOrEqual(1);
      expect(b.from + b.span - 1).toBeLessThanOrEqual(7);
    }
  });
});

describe("weekLabel", () => {
  it("ne nomme le mois qu'une fois dans une semaine entière", () => {
    expect(weekLabel(LUNDI)).toBe("2 – 8 février");
  });

  // « 30 – 5 février » laisserait croire que le 30 est en février.
  it("nomme les deux mois quand la semaine est à cheval", () => {
    expect(weekLabel(new Date(2026, 0, 26))).toBe("26 janvier – 1 février");
  });
});
