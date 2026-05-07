import { ref, computed, onMounted, onUnmounted } from "vue";
import type { Ticket, TicketDetail } from "../types";
import { useToast } from "./useToast";
import { ticketsService, type BulkDeleteParams } from "@/services/ticketsService";
import { on as onWsEvent } from "@/api/events";

export function useTickets() {
  const { error: showError } = useToast();
  const tickets = ref<Ticket[]>([]);
  const loading = ref(true);
  const filterStatus = ref("all");
  const filterPriority = ref("all");
  const filterAuthor = ref("");
  const filterFrom = ref("");
  const filterTo = ref("");
  const bulkDeleting = ref(false);

  const filteredTickets = computed(() =>
    tickets.value.filter((t) => {
      if (filterStatus.value !== "all" && t.status !== filterStatus.value) return false;
      if (filterPriority.value !== "all" && t.priority !== filterPriority.value) return false;
      if (filterAuthor.value.trim()) {
        const q = filterAuthor.value.trim().toLowerCase();
        const matchId = String(t.author_id ?? "").toLowerCase().includes(q);
        const matchName = String(t.author_name ?? "").toLowerCase().includes(q);
        if (!matchId && !matchName) return false;
      }
      if (filterFrom.value) {
        const from = new Date(filterFrom.value).getTime();
        if (new Date(t.created_at).getTime() < from) return false;
      }
      if (filterTo.value) {
        // inclusive fin de journee
        const to = new Date(filterTo.value).getTime() + 86400000;
        if (new Date(t.created_at).getTime() >= to) return false;
      }
      return true;
    }),
  );

  const hasActiveFilters = computed(
    () =>
      filterStatus.value !== "all" ||
      filterPriority.value !== "all" ||
      filterAuthor.value.trim() !== "" ||
      filterFrom.value !== "" ||
      filterTo.value !== "",
  );

  const openCount = computed(() => tickets.value.filter((t) => t.status === "open").length);
  const pendingCount = computed(() => tickets.value.filter((t) => t.status === "pending").length);

  const error = ref<string | null>(null);

  async function fetchTickets() {
    loading.value = true;
    error.value = null;
    try {
      tickets.value = await ticketsService.getAll();
    } catch (e) {
      error.value = "Impossible de charger les tickets. Verifiez la connexion au serveur.";
      console.error("Echec du chargement des tickets :", e);
      showError("Impossible de charger les tickets. Verifiez la connexion au serveur.");
    } finally {
      loading.value = false;
    }
  }

  async function bulkDelete(params: BulkDeleteParams): Promise<number> {
    bulkDeleting.value = true;
    try {
      const result = await ticketsService.bulkDelete(params);
      await fetchTickets();
      return result.deleted;
    } finally {
      bulkDeleting.value = false;
    }
  }

  function resetFilters() {
    filterStatus.value = "all";
    filterPriority.value = "all";
    filterAuthor.value = "";
    filterFrom.value = "";
    filterTo.value = "";
  }

  onMounted(fetchTickets);

  // Phase 2 sync (cf. SYNC_DISCORD_WEB_DESIGN.md) : refresh automatique
  // sur les events tickets emis par l API ou par d autres sources (bot
  // Discord, autre admin). Le bus WS local republie via @/api/events.
  const refreshOnEvent = () => {
    fetchTickets();
  };
  const offClosed = onWsEvent("ws:ticket_closed", refreshOnEvent);
  const offCreated = onWsEvent("ws:ticket_created", refreshOnEvent);
  const offAssigned = onWsEvent("ws:ticket_assigned", refreshOnEvent);
  const offStatus = onWsEvent("ws:ticket_status_updated", refreshOnEvent);
  onUnmounted(() => {
    offClosed();
    offCreated();
    offAssigned();
    offStatus();
  });

  return {
    tickets,
    filteredTickets,
    loading,
    error,
    filterStatus,
    filterPriority,
    filterAuthor,
    filterFrom,
    filterTo,
    hasActiveFilters,
    openCount,
    pendingCount,
    bulkDeleting,
    fetchTickets,
    bulkDelete,
    resetFilters,
  };
}

export function useTicketDetail() {
  const { success, error: showError } = useToast();
  const detail = ref<TicketDetail | null>(null);
  const loading = ref(false);
  const replying = ref(false);
  const error = ref<string | null>(null);

  async function fetchDetail(id: string) {
    loading.value = true;
    error.value = null;
    try {
      detail.value = await ticketsService.getDetail(id);
    } catch (e) {
      error.value = String(e);
      showError("Erreur lors du chargement du detail du ticket.");
    } finally {
      loading.value = false;
    }
  }

  async function reply(ticketId: string, content: string) {
    replying.value = true;
    error.value = null;
    try {
      await ticketsService.reply(ticketId, content);
      await fetchDetail(ticketId);
      success("Reponse envoyee avec succes.");
    } catch (e) {
      error.value = String(e);
      showError("Erreur lors de l'envoi de la reponse.");
    } finally {
      replying.value = false;
    }
  }

  async function close(id: string) {
    try {
      await ticketsService.close(id);
      if (detail.value) detail.value.ticket.status = "closed";
      success("Ticket ferme avec succes.");
    } catch (e) {
      error.value = String(e);
      showError("Erreur lors de la fermeture du ticket.");
    }
  }

  async function assign(id: string, assignee: string) {
    try {
      await ticketsService.assign(id, assignee);
      if (detail.value) detail.value.ticket.assigned_to = assignee;
      success("Ticket assigne avec succes.");
    } catch (e) {
      error.value = String(e);
      showError("Erreur lors de l'assignation du ticket.");
    }
  }

  return { detail, loading, replying, error, fetchDetail, reply, close, assign };
}
