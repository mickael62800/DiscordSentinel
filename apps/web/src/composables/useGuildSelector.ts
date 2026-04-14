import { storeToRefs } from "pinia";
import { useGuildSelectorStore } from "@/stores/guildSelectorStore";
import { useToast } from "./useToast";

export function useGuildSelector() {
  const store = useGuildSelectorStore();
  const { error: showError } = useToast();
  const { guilds, selectedGuildId, selectedGuild, guildIdFilter, loading } = storeToRefs(store);

  async function fetchGuilds() {
    await store.fetchGuilds();
    if (store.error) showError("Erreur lors du chargement des serveurs.");
  }

  return {
    guilds,
    selectedGuildId,
    selectedGuild,
    guildIdFilter,
    loading,
    fetchGuilds,
    selectGuild: store.selectGuild,
  };
}
