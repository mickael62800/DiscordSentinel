import { ref, watch } from "vue";
import type { GuildMember } from "../types";
import { useGuildSelector } from "./useGuildSelector";

const members = ref<GuildMember[]>([]);
const loading = ref(false);
const loaded = ref(false);

export function useGuildMembers() {
  const { selectedGuildId } = useGuildSelector();

  async function fetchMembers() {
    if (!selectedGuildId.value) return;
    if (loaded.value) return; // Ne charger qu'une fois

    loading.value = true;
    try {
      const resp = await fetch(`http://localhost:3000/api/guilds/${selectedGuildId.value}/members`);
      if (resp.ok) {
        members.value = await resp.json();
        loaded.value = true;
      }
    } catch (e) {
      console.error("Erreur chargement membres:", e);
    } finally {
      loading.value = false;
    }
  }

  // Recharger si le serveur change
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
