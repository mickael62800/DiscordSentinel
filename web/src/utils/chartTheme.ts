/**
 * Theme partage pour tous les graphiques Chart.js du dashboard.
 *
 * Objectif : centraliser UNE seule fois les couleurs d'axes / grille / legende
 * / tooltip et la palette categorielle, au lieu de les redefinir inline dans
 * chaque composant (ce qui provoquait deux conventions divergentes).
 *
 * Les couleurs de "chrome" (ticks, grille, texte, fond tooltip) sont lues au
 * runtime depuis les variables CSS du theme (:root de global.css) pour suivre
 * automatiquement un eventuel mode clair/sombre. Fallbacks codes en dur si la
 * variable est absente (SSR / tests / var supprimee).
 *
 * Framework-light : plain TS, aucun import Vue.
 */

/** Lit une variable CSS sur :root, avec fallback si vide/indisponible. */
function cssVar(name: string, fallback: string): string {
  if (typeof document === "undefined") return fallback;
  const v = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return v || fallback;
}

/**
 * Palette categorielle brand-neutral (~8 couleurs) lisible en clair ET sombre.
 * On garde les accents "Discord-ish" quand ils passent bien, mais definis ICI
 * une seule fois. L'ordre est choisi pour maximiser le contraste entre voisins.
 */
export const palette: string[] = [
  "#5865f2", // indigo (accent Discord)
  "#57f287", // vert
  "#faa61a", // orange ambre
  "#ed4245", // rouge
  "#5bc0eb", // cyan
  "#a855f7", // violet
  "#fee75c", // jaune
  "#eb459e", // rose/magenta
];

/** Couleur categorielle a l'index i (cycle si i depasse la palette). */
export function colorAt(i: number): string {
  return palette[((i % palette.length) + palette.length) % palette.length]!;
}

/**
 * Couleurs de severite semantiques (partagees B2 SecurityStatsGrid et autres).
 * critical=rouge, high=orange, medium=ambre, low=vert/bleu.
 */
export const severityColors = {
  critical: "#ed4245",
  high: "#faa61a",
  medium: "#fee75c",
  low: "#57f287",
  info: "#5bc0eb",
} as const;

/** Couleurs de chrome lues depuis le theme (recalculees a chaque appel). */
function themeColors() {
  return {
    text: cssVar("--text-secondary", "#9495b0"),
    textStrong: cssVar("--text-primary", "#e8e8f0"),
    grid: withAlpha(cssVar("--border", "#3a3b5c"), 0.5),
    tooltipBg: cssVar("--bg-card", "#2a2b4a"),
    border: cssVar("--border", "#3a3b5c"),
  };
}

/** Transforme un hex (#rrggbb) en rgba avec l'alpha demande. */
function withAlpha(hex: string, alpha: number): string {
  const h = hex.replace("#", "").trim();
  if (h.length !== 6) return hex;
  const r = parseInt(h.slice(0, 2), 16);
  const g = parseInt(h.slice(2, 4), 16);
  const b = parseInt(h.slice(4, 6), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/** Applique un alpha a une couleur de la palette (pour les fills). */
export function fillColor(color: string, alpha = 0.15): string {
  return withAlpha(color, alpha);
}

/** Deep-merge minimaliste (objets plain uniquement) pour appliquer overrides. */
function merge<T>(base: T, overrides?: Partial<T>): T {
  if (!overrides) return base;
  const out: Record<string, unknown> = Array.isArray(base)
    ? ([...(base as unknown[])] as unknown as Record<string, unknown>)
    : { ...(base as Record<string, unknown>) };
  for (const key of Object.keys(overrides)) {
    const o = (overrides as Record<string, unknown>)[key];
    const b = (base as Record<string, unknown>)[key];
    out[key] =
      o && typeof o === "object" && !Array.isArray(o) && b && typeof b === "object"
        ? merge(b, o)
        : o;
  }
  return out as T;
}

type AnyOptions = Record<string, unknown>;

function baseChrome() {
  const c = themeColors();
  return {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: { labels: { color: c.text, font: { size: 11 } } },
      tooltip: {
        backgroundColor: c.tooltipBg,
        titleColor: c.textStrong,
        bodyColor: c.text,
        borderColor: c.border,
        borderWidth: 1,
        padding: 10,
        cornerRadius: 8,
      },
    },
  };
}

/** Options pour un graphique en courbes (time-series / tendances). */
export function makeLineOptions(overrides?: AnyOptions): AnyOptions {
  const c = themeColors();
  const base = {
    ...baseChrome(),
    scales: {
      x: {
        ticks: { color: c.text, font: { size: 10 } },
        grid: { color: c.grid },
      },
      y: {
        ticks: { color: c.text, font: { size: 10 } },
        grid: { color: c.grid },
        beginAtZero: true,
      },
    },
    interaction: { mode: "nearest" as const, axis: "x" as const, intersect: false },
  };
  return merge(base, overrides);
}

/**
 * Options pour un graphique en barres.
 * horizontal=true -> barres horizontales (indexAxis "y", grille Y masquee).
 */
export function makeBarOptions(overrides?: AnyOptions, horizontal = false): AnyOptions {
  const c = themeColors();
  const base: AnyOptions = {
    ...baseChrome(),
    plugins: { ...baseChrome().plugins, legend: { display: false } },
    scales: horizontal
      ? {
          x: {
            ticks: { color: c.text, font: { size: 10 } },
            grid: { color: c.grid },
            beginAtZero: true,
          },
          y: {
            ticks: { color: c.text, font: { size: 11 } },
            grid: { display: false },
          },
        }
      : {
          x: {
            ticks: { color: c.text, font: { size: 10 } },
            grid: { color: c.grid },
          },
          y: {
            ticks: { color: c.text, font: { size: 10 } },
            grid: { color: c.grid },
            beginAtZero: true,
          },
        },
  };
  if (horizontal) base.indexAxis = "y";
  return merge(base, overrides);
}

/** Options pour un graphique en anneau (doughnut). */
export function makeDoughnutOptions(overrides?: AnyOptions): AnyOptions {
  const c = themeColors();
  const base = {
    responsive: true,
    maintainAspectRatio: false,
    cutout: "62%",
    plugins: {
      legend: {
        position: "bottom" as const,
        labels: { color: c.text, font: { size: 11 }, padding: 12, boxWidth: 12 },
      },
      tooltip: baseChrome().plugins.tooltip,
    },
  };
  return merge(base, overrides);
}
