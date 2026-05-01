/**
 * Enregistrement centralise de Chart.js : appele une seule fois (au lazy
 * load du chunk vendor-charts), reutilise par toutes les pages graphiques.
 *
 * Avant : chaque page (StatsPage, ModstatsPage) faisait son propre register()
 * avec exactement les memes elements -> code redondant et risque de divergence.
 * Maintenant : import unique cote consommateur -> registerChartJs().
 */

import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  BarElement,
  ArcElement,
  Title,
  Tooltip,
  Legend,
  Filler,
} from "chart.js";

let registered = false;

/**
 * Enregistre les elements Chart.js dont les pages graphiques ont besoin.
 * Idempotent : appels multiples = no-op apres le premier.
 */
export function registerChartJs(): void {
  if (registered) return;
  ChartJS.register(
    CategoryScale,
    LinearScale,
    PointElement,
    LineElement,
    BarElement,
    ArcElement,
    Title,
    Tooltip,
    Legend,
    Filler,
  );
  registered = true;
}

export { ChartJS };
