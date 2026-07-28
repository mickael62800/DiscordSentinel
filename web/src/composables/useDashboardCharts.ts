import { ref, onMounted, watch, type Ref } from "vue";
import type { DailyActivity, TopUser } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { dashboardChartsService } from "@/services/dashboardChartsService";

/**
 * Si `externalDays` est fourni, la composable l'utilise comme source
 * unique (utile quand plusieurs sections partagent un sélecteur de
 * période global). Sinon, elle maintient son propre `days`.
 */
export function useDashboardCharts(externalDays?: Ref<number>) {
  const activity = ref<DailyActivity[]>([]);
  const topUsers = ref<TopUser[]>([]);
  const loading = ref(true);
  const error = ref<string | null>(null);
  const days = externalDays ?? ref(30);
  const { guildIdFilter } = useGuildSelector();

  async function fetchAll() {
    loading.value = true;
    error.value = null;
    try {
      const [activityData, usersData] = await Promise.all([
        dashboardChartsService.getActivityTrend(guildIdFilter.value ?? null, days.value),
        guildIdFilter.value
          ? dashboardChartsService.getTopUsers(guildIdFilter.value, 10)
          : Promise.resolve([]),
      ]);
      activity.value = fillMissingDays(activityData, days.value);
      topUsers.value = usersData;
    } catch (e) {
      error.value = String(e);
      activity.value = [];
      topUsers.value = [];
      console.error("Erreur chargement dashboard:", e);
    } finally {
      loading.value = false;
    }
  }

  onMounted(fetchAll);
  watch([guildIdFilter, days], fetchAll);

  return { activity, topUsers, loading, error, days, fetchAll };
}

/**
 * Comble les jours manquants dans l'historique d'activite : l'API ne
 * renvoie que les jours qui ont au moins une ligne en base, donc un
 * serveur calme cree des trous dans les graphiques. On normalise sur
 * une plage continue de `days` jours en remplissant les manquants par
 * des zeros.
 */
function fillMissingDays(rows: DailyActivity[], days: number): DailyActivity[] {
  const byDay = new Map<string, DailyActivity>();
  for (const r of rows) byDay.set(dayKey(r.day), r);

  const result: DailyActivity[] = [];
  const today = new Date();
  today.setHours(0, 0, 0, 0);

  for (let i = days - 1; i >= 0; i--) {
    const d = new Date(today);
    d.setDate(d.getDate() - i);
    const key = dayKey(d.toISOString());
    const existing = byDay.get(key);
    if (existing) {
      result.push(existing);
    } else {
      result.push({
        day: key,
        messages: 0,
        voice_minutes: 0,
        active_members: 0,
        new_members: 0,
        leaves: 0,
        infractions: 0,
        warns: 0,
        mutes: 0,
        bans: 0,
      });
    }
  }
  return result;
}

function dayKey(iso: string): string {
  return iso.slice(0, 10);
}
