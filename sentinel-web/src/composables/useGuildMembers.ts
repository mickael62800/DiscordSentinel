import { ref, watch } from "vue";
import type { GuildMember } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import { guildsService } from "@/services/guildsService";

const members = ref<GuildMember[]>([]);
const loading = ref(false);
const loaded = ref(false);

export function useGuildMembers() {
  const { selectedGuildId } = useGuildSelector();
  const { error: showError } = useToast();

  async function fetchMembers() {
    if (!selectedGuildId.value) return;
    if (loaded.value) return;

    loading.value = true;
    try {
      members.value = await guildsService.getMembers(selectedGuildId.value);
      loaded.value = true;
    } catch (e) {
      console.error("Erreur lors du chargement des membres :", e);
      showError("Erreur lors du chargement des membres.");
    } finally {
      loading.value = false;
    }
  }

  watch(selectedGuildId, () => {
    loaded.value = false;
    members.value = [];
    fetchMembers();
  }, { immediate: true });

  function searchMembers(query: string): GuildMember[] {
    if (!query || query.length < 1) return [];
    const q = query.toLowerCase();
    return members.value
      .filter((m) =>
        m.username.toLowerCase().includes(q) ||
        (m.display_name && m.display_name.toLowerCase().includes(q)) ||
        m.id.includes(q)
      )
      .slice(0, 10);
  }

  return { members, loading, fetchMembers, searchMembers };
}
