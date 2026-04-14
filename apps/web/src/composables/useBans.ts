import { ref, computed, onMounted, watch } from "vue";
import type { Infraction, ConfirmedBan } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import { infractionsService } from "@/services/infractionsService";
import { moderationService } from "@/services/moderationService";

export function useBans() {
  const { guildIdFilter } = useGuildSelector();
  const { success, error: showError } = useToast();
  const loading = ref(true);
  const searchQuery = ref("");

  const banProposals = ref<Infraction[]>([]);
  const confirmedBans = ref<ConfirmedBan[]>([]);

  async function fetchBans() {
    loading.value = true;
    try {
      const guildId = guildIdFilter.value ?? null;
      const [infractions, bans] = await Promise.all([
        infractionsService.getAll(guildId),
        moderationService.getConfirmedBans(guildId),
      ]);
      // Le backend retourne deja les utilisateurs actuellement bannis
      // (deduplique par target_id, derniere action = ban, pas un unban).
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
      console.error("Erreur lors du chargement des bans :", e);
      showError("Erreur lors du chargement des bannissements.");
    } finally {
      loading.value = false;
    }
  }

  const filteredProposals = computed(() => {
    if (!searchQuery.value) return banProposals.value;
    const q = searchQuery.value.toLowerCase();
    return banProposals.value.filter(
      (b) => b.username.toLowerCase().includes(q) || b.user_id.includes(q) || b.reason.toLowerCase().includes(q),
    );
  });

  const filteredConfirmed = computed(() => {
    if (!searchQuery.value) return confirmedBans.value;
    const q = searchQuery.value.toLowerCase();
    return confirmedBans.value.filter(
      (b) => b.target_name.toLowerCase().includes(q) || b.target_id.includes(q) || b.reason.toLowerCase().includes(q),
    );
  });

  const totalProposals = computed(() => banProposals.value.length);
  const totalConfirmed = computed(() => confirmedBans.value.length);

  const banning = ref(false);

  async function executeBan(guildId: string, userId: string, reason: string) {
    banning.value = true;
    try {
      await moderationService.executeBan(guildId, userId, reason);
      await fetchBans();
      success("Utilisateur banni avec succes.");
    } catch (e) {
      console.error("Erreur lors de l'execution du ban :", e);
      showError("Erreur lors du bannissement de l'utilisateur.");
      throw e;
    } finally {
      banning.value = false;
    }
  }

  async function executeUnban(guildId: string, userId: string) {
    banning.value = true;
    try {
      await moderationService.executeUnban(guildId, userId);
      await fetchBans();
      success("Utilisateur debanni avec succes.");
    } catch (e) {
      console.error("Erreur lors de l'execution du deban :", e);
      showError("Erreur lors du debannissement de l'utilisateur.");
      throw e;
    } finally {
      banning.value = false;
    }
  }

  onMounted(fetchBans);
  watch(guildIdFilter, fetchBans);

  return {
    filteredProposals, filteredConfirmed, totalProposals, totalConfirmed,
    loading, banning, searchQuery, fetchBans, executeBan, executeUnban,
  };
}
