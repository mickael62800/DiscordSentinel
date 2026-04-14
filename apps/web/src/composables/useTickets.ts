import { ref, computed, onMounted } from "vue";
import type { Ticket, TicketDetail } from "../types";
import { useToast } from "./useToast";
import { ticketsService } from "@/services/ticketsService";

export function useTickets() {
  const { error: showError } = useToast();
  const tickets = ref<Ticket[]>([]);
  const loading = ref(true);
  const filterStatus = ref("all");
  const filterPriority = ref("all");

  const filteredTickets = computed(() => tickets.value.filter((t) => {
    if (filterStatus.value !== "all" && t.status !== filterStatus.value) return false;
    if (filterPriority.value !== "all" && t.priority !== filterPriority.value) return false;
    return true;
  }));

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

  onMounted(fetchTickets);

  return { tickets, filteredTickets, loading, error, filterStatus, filterPriority, openCount, pendingCount, fetchTickets };
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
