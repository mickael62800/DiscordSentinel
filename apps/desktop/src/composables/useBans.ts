import { ref, computed } from "vue";
import { useFetch } from "./useFetch";
import type { Infraction } from "../types";

export function useBans() {
  const { data: infractions, loading, refresh: fetchBans } = useFetch<Infraction[]>("get_infractions", []);
  const searchQuery = ref("");
  const filterServer = ref("all");

  const bans = computed(() =>
    infractions.value.filter((i) => i.infraction_type === "ban")
  );

  const filteredBans = computed(() => {
    return bans.value.filter((b) => {
      if (filterServer.value !== "all" && b.server !== filterServer.value) return false;
      if (searchQuery.value) {
        const q = searchQuery.value.toLowerCase();
        return (
          b.username.toLowerCase().includes(q) ||
          b.user_id.includes(q) ||
          b.reason.toLowerCase().includes(q)
        );
      }
      return true;
    });
  });

  const servers = computed(() => Array.from(new Set(bans.value.map((b) => b.server))));
  const totalBans = computed(() => bans.value.length);

  return { bans, filteredBans, servers, totalBans, loading, searchQuery, filterServer, fetchBans };
}
