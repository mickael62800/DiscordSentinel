import { ref, computed } from "vue";
import type { Member, MemberSummary, UserDossier, WatchedUser } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import { membersService } from "@/services/membersService";
import { watchedUsersService } from "@/services/watchedUsersService";
import { userActivityService, type UserActivity } from "@/services/userActivityService";

const members = ref<Member[]>([]);
const watchedUsers = ref<WatchedUser[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const search = ref("");
const sortBy = ref<"username" | "joined_at">("username");

const selectedMember = ref<MemberSummary | null>(null);
const loadingSummary = ref(false);

const dossier = ref<UserDossier | null>(null);
const dossierLoading = ref(false);

const activityTimeline = ref<UserActivity[]>([]);

export function useMembers() {
  const { selectedGuildId } = useGuildSelector();
  const { error: showError } = useToast();

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
        membersService.getAll(selectedGuildId.value),
        watchedUsersService.getAll(selectedGuildId.value),
      ]);
      members.value = m;
      watchedUsers.value = w;
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function selectMember(userId: string) {
    if (!selectedGuildId.value) return;
    loadingSummary.value = true;
    dossier.value = null;
    try {
      selectedMember.value = await membersService.getSummary(selectedGuildId.value, userId);
    } catch (e) {
      error.value = String(e);
      showError("Erreur lors du chargement du resume du membre.");
    } finally {
      loadingSummary.value = false;
    }
  }

  async function fetchDossier(userId: string) {
    if (!selectedGuildId.value) return;
    const guildId = selectedGuildId.value;
    dossierLoading.value = true;
    try {
      const [d, activity] = await Promise.all([
        watchedUsersService.getDossier(guildId, userId),
        userActivityService.list(guildId, userId, { limit: 100 }).catch(() => []),
      ]);
      dossier.value = d;
      activityTimeline.value = activity;
    } catch {
      dossier.value = null;
      activityTimeline.value = [];
    } finally {
      dossierLoading.value = false;
    }
  }

  async function addToWatch(userId: string, username: string) {
    if (!selectedGuildId.value) return;
    await watchedUsersService.add(
      selectedGuildId.value,
      userId,
      username,
      "Ajout manuel depuis la page Membres",
    );
    watchedUsers.value = await watchedUsersService.getAll(selectedGuildId.value);
  }

  async function removeFromWatch(userId: string) {
    if (!selectedGuildId.value) return;
    // On laisse l'erreur remonter : le caller (MembersPage.unwatch) affiche
    // lui-meme le toast de succes/erreur, sinon on avait un double toast.
    await watchedUsersService.remove(selectedGuildId.value, userId);
    watchedUsers.value = watchedUsers.value.filter((u) => u.user_id !== userId);
  }

  async function resetMember(userId: string): Promise<Record<string, number>> {
    if (!selectedGuildId.value) throw new Error("Aucun serveur selectionne");
    const result = await membersService.resetMember(selectedGuildId.value, userId);
    // Refresh interne : retire du watchedSet si present.
    watchedUsers.value = watchedUsers.value.filter((u) => u.user_id !== userId);
    return result.totals;
  }

  function closeMember() {
    selectedMember.value = null;
    dossier.value = null;
    activityTimeline.value = [];
  }

  return {
    members, filteredMembers, loading, error, search, sortBy,
    selectedMember, loadingSummary,
    dossier, dossierLoading,
    activityTimeline,
    isWatched,
    fetchMembers, selectMember,
    fetchDossier, addToWatch, removeFromWatch, resetMember, closeMember,
  };
}
