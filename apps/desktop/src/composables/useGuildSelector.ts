import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Guild } from "../types";

const guilds = ref<Guild[]>([]);
const selectedGuildId = ref<string | null>(null);
const loading = ref(false);

export function useGuildSelector() {
  const selectedGuild = computed(() =>
    guilds.value.find((g) => g.guild_id === selectedGuildId.value) ?? null
  );

  const guildIdFilter = computed(() => selectedGuildId.value ?? undefined);

  async function fetchGuilds() {
    loading.value = true;
    try {
      guilds.value = await invoke<Guild[]>("get_guilds");
      // Restaurer la selection depuis le localStorage
      const saved = localStorage.getItem("sentinel_selected_guild");
      if (saved && guilds.value.some((g) => g.guild_id === saved)) {
        selectedGuildId.value = saved;
      }
    } catch (e) {
      console.error("Erreur chargement des serveurs:", e);
    } finally {
      loading.value = false;
    }
  }

  function selectGuild(guildId: string | null) {
    selectedGuildId.value = guildId;
    if (guildId) {
      localStorage.setItem("sentinel_selected_guild", guildId);
    } else {
      localStorage.removeItem("sentinel_selected_guild");
    }
  }

  return {
    guilds,
    selectedGuildId,
    selectedGuild,
    guildIdFilter,
    loading,
    fetchGuilds,
    selectGuild,
  };
}
