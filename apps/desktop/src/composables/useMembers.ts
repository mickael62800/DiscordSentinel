import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Member, MemberSummary, UserConductPoints, ConductPointsLog, ConductConfig, UserDossier, WatchedUser } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { getApiBaseUrl } from "../utils/api";
import { useToast } from "./useToast";

const members = ref<Member[]>([]);
const watchedUsers = ref<WatchedUser[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const search = ref("");
const sortBy = ref<"username" | "joined_at">("username");

// Detail
const selectedMember = ref<MemberSummary | null>(null);
const loadingSummary = ref(false);

// Conduct
const conductConfig = ref<ConductConfig | null>(null);
const conductPoints = ref<UserConductPoints | null>(null);
const conductLog = ref<ConductPointsLog[]>([]);
const conductLoading = ref(false);

// Surveillance (dossier)
const dossier = ref<UserDossier | null>(null);
const dossierLoading = ref(false);

export function useMembers() {
  const { selectedGuildId } = useGuildSelector();
  const { success, error: showError } = useToast();

  const watchedSet = computed(() => new Set(watchedUsers.value.map((u) => u.user_id)));

  function isWatched(userId: string): boolean {
    return watchedSet.value.has(userId);
  }

  const filteredMembers = computed(() => {
    let result = members.value;

    if (search.value) {
      const q = search.value.toLowerCase();
      result = result.filter(
        (m) =>
          m.username.toLowerCase().includes(q) ||
          (m.display_name && m.display_name.toLowerCase().includes(q)) ||
          m.user_id.includes(q),
      );
    }

    result = [...result].sort((a, b) => {
      // Watched users always first
      const aW = watchedSet.value.has(a.user_id) ? 0 : 1;
      const bW = watchedSet.value.has(b.user_id) ? 0 : 1;
      if (aW !== bW) return aW - bW;

      if (sortBy.value === "joined_at") {
        return (a.joined_at ?? "").localeCompare(b.joined_at ?? "");
      }
      return a.username.localeCompare(b.username);
    });

    return result;
  });

  async function fetchMembers() {
    if (!selectedGuildId.value) return;
    loading.value = true;
    error.value = null;
    try {
      const [m, w] = await Promise.all([
        invoke<Member[]>("get_members", { guildId: selectedGuildId.value }),
        invoke<WatchedUser[]>("get_watched_users", { guildId: selectedGuildId.value }),
      ]);
      members.value = m;
      watchedUsers.value = w;
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function fetchConductConfig() {
    if (!selectedGuildId.value) return;
    try {
      conductConfig.value = await invoke<ConductConfig>("get_conduct_config", {
        guildId: selectedGuildId.value,
      });
    } catch {
      // Config may not exist yet
    }
  }

  async function selectMember(userId: string) {
    if (!selectedGuildId.value) return;
    loadingSummary.value = true;
    dossier.value = null;
    conductPoints.value = null;
    conductLog.value = [];
    try {
      selectedMember.value = await invoke<MemberSummary>("get_member_summary", {
        guildId: selectedGuildId.value,
        userId,
      });
    } catch (e) {
      error.value = String(e);
      showError("Erreur lors du chargement du resume du membre.");
    } finally {
      loadingSummary.value = false;
    }
  }

  async function fetchConductDetail(userId: string) {
    if (!selectedGuildId.value) return;
    conductLoading.value = true;
    try {
      conductPoints.value = await invoke<UserConductPoints>("get_conduct_points", {
        guildId: selectedGuildId.value,
        userId,
      });
      conductLog.value = await invoke<ConductPointsLog[]>("get_conduct_log", {
        guildId: selectedGuildId.value,
        userId,
      });
    } catch {
      // User may not have conduct points yet
      conductPoints.value = null;
      conductLog.value = [];
    } finally {
      conductLoading.value = false;
    }
  }

  async function adjustPoints(userId: string, amount: number, reason: string) {
    if (!selectedGuildId.value) return;
    try {
      await invoke("adjust_conduct_points", {
        guildId: selectedGuildId.value,
        userId,
        amount,
        reason,
      });
      await fetchConductDetail(userId);
      success("Points de conduite ajustes avec succes.");
    } catch (e) {
      console.error("Erreur lors de l'ajustement des points de conduite :", e);
      showError("Erreur lors de l'ajustement des points de conduite.");
    }
  }

  async function fetchDossier(userId: string) {
    if (!selectedGuildId.value) return;
    dossierLoading.value = true;
    try {
      dossier.value = await invoke<UserDossier>("get_user_dossier", {
        guildId: selectedGuildId.value,
        userId,
      });
    } catch {
      dossier.value = null;
    } finally {
      dossierLoading.value = false;
    }
  }

  async function addToWatch(userId: string, username: string) {
    if (!selectedGuildId.value) return;
    const baseUrl = await getApiBaseUrl();
    const resp = await fetch(`${baseUrl}/api/watched-users`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        guild_id: selectedGuildId.value,
        user_id: userId,
        username,
        reason: "Ajout manuel depuis la page Membres",
      }),
    });
    if (!resp.ok) throw new Error("Erreur ajout surveillance");
    // Rafraichir la liste des surveilles
    watchedUsers.value = await invoke<WatchedUser[]>("get_watched_users", { guildId: selectedGuildId.value });
  }

  async function removeFromWatch(userId: string) {
    if (!selectedGuildId.value) return;
    try {
      await invoke("remove_watched_user", {
        guildId: selectedGuildId.value,
        userId,
      });
      // Retirer localement pour mise a jour immediate
      watchedUsers.value = watchedUsers.value.filter((u) => u.user_id !== userId);
      success("Utilisateur retire de la surveillance.");
    } catch (e) {
      console.error("Erreur lors du retrait de la surveillance :", e);
      showError("Erreur lors du retrait de la surveillance.");
    }
  }

  function closeMember() {
    selectedMember.value = null;
    dossier.value = null;
    conductPoints.value = null;
    conductLog.value = [];
  }

  return {
    members,
    filteredMembers,
    loading,
    error,
    search,
    sortBy,
    selectedMember,
    loadingSummary,
    conductConfig,
    conductPoints,
    conductLog,
    conductLoading,
    dossier,
    dossierLoading,
    isWatched,
    fetchMembers,
    fetchConductConfig,
    selectMember,
    fetchConductDetail,
    adjustPoints,
    fetchDossier,
    addToWatch,
    removeFromWatch,
    closeMember,
  };
}
