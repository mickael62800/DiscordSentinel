import { ref, computed, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Infraction, ConfirmedBan } from "../types";
import { useGuildSelector } from "./useGuildSelector";

export function useBans() {
  const { guildIdFilter } = useGuildSelector();
  const loading = ref(true);
  const searchQuery = ref("");

  const banProposals = ref<Infraction[]>([]);
  const confirmedBans = ref<ConfirmedBan[]>([]);

  async function fetchBans() {
    loading.value = true;
    try {
      const guildId = guildIdFilter.value ?? null;
      const [infractions, bans] = await Promise.all([
        invoke<Infraction[]>("get_infractions", { guildId }),
        invoke<ConfirmedBan[]>("get_confirmed_bans", { guildId }),
      ]);
      confirmedBans.value = bans;
      const bannedIds = new Set(bans.map((b) => `${b.guild_id}:${b.target_id}`));
      const allBanProposals = infractions.filter((i) => i.infraction_type === "ban");
      const seen = new Set<string>();
      banProposals.value = allBanProposals.filter((b) => {
        const key = `${b.server}:${b.user_id}`;
        if (seen.has(key) || bannedIds.has(key)) return false;
        seen.add(key);
        return true;
      });
    } catch (e) {
      console.error("Erreur chargement bans:", e);
    } finally {
      loading.value = false;
    }
  }

  const filteredProposals = computed(() => {
    if (!searchQuery.value) return banProposals.value;
    const q = searchQuery.value.toLowerCase();
    return banProposals.value.filter(
      (b) =>
        b.username.toLowerCase().includes(q) ||
        b.user_id.includes(q) ||
        b.reason.toLowerCase().includes(q),
    );
  });

  const filteredConfirmed = computed(() => {
    if (!searchQuery.value) return confirmedBans.value;
    const q = searchQuery.value.toLowerCase();
    return confirmedBans.value.filter(
      (b) =>
        b.target_name.toLowerCase().includes(q) ||
        b.target_id.includes(q) ||
        b.reason.toLowerCase().includes(q),
    );
  });

  const totalProposals = computed(() => banProposals.value.length);
  const totalConfirmed = computed(() => confirmedBans.value.length);

  const banning = ref(false);

  async function executeBan(guildId: string, userId: string, reason: string) {
    banning.value = true;
    try {
      await invoke("execute_ban", { guildId, userId, reason });
      await fetchBans();
    } catch (e) {
      console.error("Erreur execution ban:", e);
      throw e;
    } finally {
      banning.value = false;
    }
  }

  async function executeUnban(guildId: string, userId: string) {
    banning.value = true;
    try {
      await invoke("execute_unban", { guildId, userId });
      await fetchBans();
    } catch (e) {
      console.error("Erreur execution unban:", e);
      throw e;
    } finally {
      banning.value = false;
    }
  }

  onMounted(fetchBans);
  watch(guildIdFilter, fetchBans);

  return {
    filteredProposals,
    filteredConfirmed,
    totalProposals,
    totalConfirmed,
    loading,
    banning,
    searchQuery,
    fetchBans,
    executeBan,
    executeUnban,
  };
}
