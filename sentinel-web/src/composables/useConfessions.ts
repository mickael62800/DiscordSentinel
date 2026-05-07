import { ref, watch } from "vue";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import { useRealtimeRefresh } from "./useRealtimeRefresh";
import {
  confessionsService,
  type Confession,
  type ConfessionReply,
  type ConfessionReport,
} from "@/services/confessionsService";

// Singleton module-scoped : un cache partage entre Header / Tables / Modal.
const { selectedGuildId } = useGuildSelector();

const tab = ref<"confessions" | "reports">("confessions");
const showDeleted = ref(false);

const confessions = ref<Confession[]>([]);
const reports = ref<ConfessionReport[]>([]);
const loading = ref(false);

const repliesTarget = ref<Confession | null>(null);
const replies = ref<ConfessionReply[]>([]);

async function fetchAll() {
  const { error: toastErr } = useToast();
  if (!selectedGuildId.value) return;
  loading.value = true;
  try {
    const [c, r] = await Promise.all([
      confessionsService.list(selectedGuildId.value, showDeleted.value, 200),
      confessionsService.listReports(selectedGuildId.value, "pending", 100),
    ]);
    confessions.value = c;
    reports.value = r;
  } catch (e: unknown) {
    toastErr(`Echec chargement : ${(e as Error)?.message ?? e}`);
  } finally {
    loading.value = false;
  }
}

watch([selectedGuildId, showDeleted], fetchAll, { immediate: true });

useRealtimeRefresh(
  [
    "confession_created",
    "confession_edited",
    "confession_deleted",
    "confession_reply_created",
    "confession_reply_deleted",
    "confession_report_created",
  ],
  fetchAll,
);

export function useConfessions() {
  const { success: toastOk, error: toastErr } = useToast();

  async function showReplies(c: Confession) {
    repliesTarget.value = c;
    try {
      replies.value = await confessionsService.listReplies(c.id);
    } catch (e: unknown) {
      toastErr(`Echec replies : ${(e as Error)?.message ?? e}`);
      replies.value = [];
    }
  }

  function closeReplies() {
    repliesTarget.value = null;
    replies.value = [];
  }

  async function deleteConfession(c: Confession) {
    try {
      await confessionsService.delete(c.id, "web-admin", "Supprimee par admin via web");
      toastOk(`Confession #${c.public_number} supprimee.`);
      await fetchAll();
    } catch (e: unknown) {
      toastErr(`Echec : ${(e as Error)?.message ?? e}`);
    }
  }

  async function deleteReply(r: ConfessionReply) {
    try {
      await confessionsService.deleteReply(r.id, "web-admin");
      toastOk("Reply supprime.");
      if (repliesTarget.value) await showReplies(repliesTarget.value);
    } catch (e: unknown) {
      toastErr(`Echec : ${(e as Error)?.message ?? e}`);
    }
  }

  async function resolveReport(r: ConfessionReport, status: "resolved" | "dismissed") {
    try {
      await confessionsService.resolveReport(r.id, status, "web-admin");
      toastOk(`Signalement ${status === "resolved" ? "résolu" : "rejeté"}.`);
      await fetchAll();
    } catch (e: unknown) {
      toastErr(`Echec : ${(e as Error)?.message ?? e}`);
    }
  }

  return {
    tab, showDeleted, confessions, reports, loading,
    repliesTarget, replies,
    fetchAll, showReplies, closeReplies,
    deleteConfession, deleteReply, resolveReport,
  };
}
