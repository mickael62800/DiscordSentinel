import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Ticket, TicketDetail } from "../types";

export function useTickets() {
  const tickets = ref<Ticket[]>([]);
  const loading = ref(true);
  const filterStatus = ref("all");
  const filterPriority = ref("all");

  const filteredTickets = computed(() => {
    return tickets.value.filter((t) => {
      if (filterStatus.value !== "all" && t.status !== filterStatus.value) return false;
      if (filterPriority.value !== "all" && t.priority !== filterPriority.value) return false;
      return true;
    });
  });

  const openCount = computed(() => tickets.value.filter((t) => t.status === "open").length);
  const pendingCount = computed(() => tickets.value.filter((t) => t.status === "pending").length);

  const error = ref<string | null>(null);

  async function fetchTickets() {
    loading.value = true;
    error.value = null;
    try {
      tickets.value = await invoke<Ticket[]>("get_tickets");
    } catch (e) {
      error.value = "Impossible de charger les tickets. Verifiez la connexion au serveur.";
      console.error("Failed to fetch tickets:", e);
    } finally {
      loading.value = false;
    }
  }

  onMounted(fetchTickets);

  return { tickets, filteredTickets, loading, error, filterStatus, filterPriority, openCount, pendingCount, fetchTickets };
}

export function useTicketDetail() {
  const detail = ref<TicketDetail | null>(null);
  const loading = ref(false);
  const replying = ref(false);
  const error = ref<string | null>(null);

  async function fetchDetail(id: string) {
    loading.value = true;
    error.value = null;
    try {
      detail.value = await invoke<TicketDetail>("get_ticket_detail", { id });
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function reply(ticketId: string, content: string) {
    replying.value = true;
    error.value = null;
    try {
      await invoke("reply_ticket", { ticketId, content });
      await fetchDetail(ticketId);
    } catch (e) {
      error.value = String(e);
    } finally {
      replying.value = false;
    }
  }

  async function close(id: string) {
    try {
      await invoke("close_ticket", { id });
      if (detail.value) detail.value.ticket.status = "closed";
    } catch (e) {
      error.value = String(e);
    }
  }

  async function assign(id: string, assignee: string) {
    try {
      await invoke("assign_ticket", { id, assignee });
      if (detail.value) detail.value.ticket.assigned_to = assignee;
    } catch (e) {
      error.value = String(e);
    }
  }

  return { detail, loading, replying, error, fetchDetail, reply, close, assign };
}
