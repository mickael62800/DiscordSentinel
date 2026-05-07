import { ref, computed } from "vue";
import { listen, type UnlistenFn } from "@/api/events-api";
import { isPermissionGranted, requestPermission, sendNotification } from "@/api/notification";
import type { Notification } from "../types";
import { useToast } from "./useToast";

const notifications = ref<Notification[]>([]);
const panelOpen = ref(false);
let listening = false;
const unlisteners: UnlistenFn[] = [];

const unreadCount = computed(() => notifications.value.filter((n) => !n.read).length);

export function useNotifications() {

  async function startListening() {
    if (listening) return;
    listening = true;

    const toast = useToast();

    const u1 = await listen<{ event: string; data: unknown }>("ws:event", (e) => {
      const wsEvent = e.payload;
      const notif = eventToNotification(wsEvent);
      if (notif) {
        notifications.value.unshift(notif);
        // Cap at 200 notifications to prevent unbounded growth
        if (notifications.value.length > 200) {
          notifications.value.splice(200);
        }
        if (notif.severity === "critical" || notif.severity === "high") {
          sendNativeNotification(notif);
        }
        // Toast automatique pour les events critiques/high
        if (notif.severity === "critical") {
          toast.error(`${notif.title} — ${notif.message}`);
        } else if (notif.severity === "high") {
          toast.warning(`${notif.title} — ${notif.message}`);
        }
      }

      // Toast pour les logs d'erreur des bots/workers
      if (wsEvent.event === "log_entry_created") {
        const data = wsEvent.data as Record<string, string>;
        if (data.level === "error") {
          toast.error(`[${data.bot ?? "systeme"}] ${data.message ?? "Erreur inconnue"}`);
        } else if (data.level === "warn") {
          toast.warning(`[${data.bot ?? "systeme"}] ${data.message ?? "Avertissement"}`);
        }
      }
    });
    unlisteners.push(u1);

    const u2 = await listen<Notification>("ws:notification", (e) => {
      const notif = e.payload;
      notifications.value.unshift(notif);
      if (notifications.value.length > 200) {
        notifications.value.length = 200;
      }
      if (notif.severity === "critical" || notif.severity === "high") {
        sendNativeNotification(notif);
      }
    });
    unlisteners.push(u2);
  }

  function stopListening() {
    for (const unlisten of unlisteners) {
      unlisten();
    }
    unlisteners.length = 0;
    listening = false;
  }

  function markAsRead(id: string) {
    const notif = notifications.value.find((n) => n.id === id);
    if (notif) notif.read = true;
  }

  function markAllAsRead() {
    notifications.value.forEach((n) => (n.read = true));
  }

  function togglePanel() {
    panelOpen.value = !panelOpen.value;
  }

  function closePanel() {
    panelOpen.value = false;
  }

  return {
    notifications,
    unreadCount,
    panelOpen,
    startListening,
    stopListening,
    markAsRead,
    markAllAsRead,
    togglePanel,
    closePanel,
  };
}

function eventToNotification(wsEvent: { event: string; data: unknown }): Notification | null {
  const now = new Date().toISOString().replace("T", " ").slice(0, 19);
  const id = `notif-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
  const data = wsEvent.data as Record<string, string>;

  switch (wsEvent.event) {
    case "infraction_new":
      return {
        id,
        notification_type: "infraction",
        title: `Nouvelle ${data.action ?? "infraction"}`,
        message: `${data.username ?? "Utilisateur"} — ${data.reason ?? "Aucune raison"}`,
        severity: data.action === "ban" ? "high" : "medium",
        read: false,
        created_at: now,
      };

    case "ticket_new":
      return {
        id,
        notification_type: "ticket",
        title: "Nouveau ticket",
        message: `${data.author_name ?? "Utilisateur"}: ${data.title ?? ""}`,
        severity: data.priority === "urgent" ? "high" : "medium",
        read: false,
        created_at: now,
      };

    case "ticket_message":
      return {
        id,
        notification_type: "ticket",
        title: "Reponse ticket",
        message: `${data.author_name ?? "Utilisateur"} a repondu au ticket`,
        severity: "low",
        read: false,
        created_at: now,
      };

    case "bot_status": {
      const online = (data as unknown as { online: boolean }).online;
      return {
        id,
        notification_type: "bot",
        title: online ? "Bot en ligne" : "Bot hors ligne",
        message: `${data.bot ?? "Bot"} est maintenant ${online ? "connecte" : "deconnecte"}`,
        severity: online ? "low" : "high",
        read: false,
        created_at: now,
      };
    }

    case "raid_detected":
      return {
        id,
        notification_type: "raid",
        title: "Raid detecte",
        message: data.message ?? "Activite suspecte detectee",
        severity: "critical",
        read: false,
        created_at: now,
      };

    case "security_event":
      return {
        id,
        notification_type: "security",
        title: `Security: ${(data.event_type ?? "event").replace("_", " ")}`,
        message: data.description ?? "Evenement de securite detecte",
        severity: data.severity === "critical" ? "critical" : data.severity === "high" ? "high" : "medium",
        read: false,
        created_at: now,
      };

    case "moderation_action":
      return {
        id,
        notification_type: "moderation",
        title: `${data.action_type ?? "Action"} applique`,
        message: `${data.moderator_name ?? "Moderateur"} → ${data.target_name ?? "user"}: ${data.reason ?? ""}`,
        severity: data.action_type === "ban" ? "high" : "medium",
        read: false,
        created_at: now,
      };

    case "log_entry_created": {
      if (data.level !== "error" && data.level !== "warn") return null;
      return {
        id,
        notification_type: "log",
        title: `${data.level === "error" ? "Erreur" : "Avertissement"} — ${data.bot ?? "systeme"}`,
        message: data.message ?? "Evenement systeme",
        severity: data.level === "error" ? "high" : "medium",
        read: false,
        created_at: now,
      };
    }

    case "user_zero_points":
      return {
        id,
        notification_type: "conduct",
        title: "Points de conduite a zero",
        message: `${data.username ?? "Utilisateur"} a atteint 0 points (action: ${data.action ?? "inconnue"})`,
        severity: "high",
        read: false,
        created_at: now,
      };

    case "strike_added": {
      const escalation = data.escalation_action;
      if (!escalation) return null;
      return {
        id,
        notification_type: "moderation",
        title: "Escalade de sanction",
        message: `${data.user_id ?? "Utilisateur"} — ${data.active_count ?? "?"} strikes actifs → ${escalation}`,
        severity: "high",
        read: false,
        created_at: now,
      };
    }

    case "watched_user_added":
      return {
        id,
        notification_type: "surveillance",
        title: "Utilisateur sous surveillance",
        message: `${data.username ?? "Utilisateur"} a ete ajoute a la liste de surveillance`,
        severity: "medium",
        read: false,
        created_at: now,
      };

    default:
      return null;
  }
}

async function sendNativeNotification(notif: Notification) {
  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      const permission = await requestPermission();
      granted = permission === "granted";
    }
    if (granted) {
      sendNotification({
        title: `Sentinel: ${notif.title}`,
        body: notif.message,
      });
    }
  } catch {
    // Native notifications not available
  }
}
