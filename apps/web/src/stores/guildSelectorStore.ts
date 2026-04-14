import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { guildsService } from "@/services/guildsService";
import type { Guild } from "@/types";

export const useGuildSelectorStore = defineStore("guildSelector", () => {
  const guilds = ref<Guild[]>([]);
  const selectedGuildId = ref<string | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const selectedGuild = computed(() =>
    guilds.value.find((g) => g.guild_id === selectedGuildId.value) ?? null,
  );
  const guildIdFilter = computed(() => selectedGuildId.value ?? undefined);

  async function fetchGuilds() {
    loading.value = true;
    error.value = null;
    try {
      guilds.value = await guildsService.getAll();
      const saved = localStorage.getItem("sentinel_selected_guild");
      if (saved && guilds.value.some((g) => g.guild_id === saved)) {
        selectedGuildId.value = saved;
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
