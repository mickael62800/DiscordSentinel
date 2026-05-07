import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { guildsService } from "@/services/guildsService";
import type { Guild } from "@/types";

const GUILDS_CACHE_KEY = "sentinel_guilds_cache";
const GUILDS_CACHE_TTL_MS = 6 * 60 * 60 * 1000; // 6h

interface GuildsCache {
  data: Guild[];
  ts: number;
}

function loadCache(): Guild[] | null {
  try {
    const raw = localStorage.getItem(GUILDS_CACHE_KEY);
    if (!raw) return null;
    const cache: GuildsCache = JSON.parse(raw);
    if (Date.now() - cache.ts > GUILDS_CACHE_TTL_MS) return null;
    return cache.data;
  } catch {
    return null;
  }
}

function saveCache(data: Guild[]): void {
  try {
    localStorage.setItem(GUILDS_CACHE_KEY, JSON.stringify({ data, ts: Date.now() } as GuildsCache));
  } catch {
    // Quota exceeded ou storage indisponible : on ignore (cache best-effort).
  }
}

export const useGuildSelectorStore = defineStore("guildSelector", () => {
  const guilds = ref<Guild[]>([]);
  const selectedGuildId = ref<string | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const selectedGuild = computed(() =>
    guilds.value.find((g) => g.guild_id === selectedGuildId.value) ?? null,
  );
  const guildIdFilter = computed(() => selectedGuildId.value ?? undefined);

  /**
   * Strategie SWR : hydrate depuis localStorage immediatement (cold-start instant),
   * puis refetch en background pour valider. TTL 6h sur le cache local.
   */
  async function fetchGuilds() {
    // 1. Hydratation instantanee depuis le cache (si valide)
    const cached = loadCache();
    if (cached && guilds.value.length === 0) {
      guilds.value = cached;
      const saved = localStorage.getItem("sentinel_selected_guild");
      if (saved && guilds.value.some((g) => g.guild_id === saved)) {
        selectedGuildId.value = saved;
      }
    }

    // 2. Refetch en background (toujours, pour valider/rafraichir)
    loading.value = guilds.value.length === 0; // ne pas afficher spinner si on a deja le cache
    error.value = null;
    try {
      const fresh = await guildsService.getAll();
      guilds.value = fresh;
      saveCache(fresh);
      const saved = localStorage.getItem("sentinel_selected_guild");
      if (saved && guilds.value.some((g) => g.guild_id === saved)) {
        selectedGuildId.value = saved;
      } else if (selectedGuildId.value && !guilds.value.some((g) => g.guild_id === selectedGuildId.value)) {
        // La guild selectionnee n'existe plus -> reset
        selectedGuildId.value = null;
      }
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  function selectGuild(guildId: string | null) {
    selectedGuildId.value = guildId;
    if (guildId) localStorage.setItem("sentinel_selected_guild", guildId);
    else localStorage.removeItem("sentinel_selected_guild");
  }

  return {
    guilds, selectedGuildId, selectedGuild, guildIdFilter,
    loading, error, fetchGuilds, selectGuild,
  };
});
