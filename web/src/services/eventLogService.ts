// Journal d'evenements du serveur Discord.
//
// Remplace les salons de logs Discord : tout ce que le bot postait dans
// #logs-membres, #logs-vocal, #logs-messages… est lu ici depuis `audit_logs`.
//
// Contrairement a `auditLogsService` (qui charge 500 entrees puis filtre en
// memoire), tout le filtrage et la pagination se font cote serveur : le
// journal est desormais la seule vue des evenements, il doit tenir sur la
// duree de retention complete.

import { httpGetWithTotal } from "@/api/http";
import type { AuditLog } from "@/types";
import { q } from "./_query";

/// Regroupement par NATURE d'evenement, tel que percu par l'utilisateur.
/// Chaque categorie correspondait a un salon de logs Discord.
export interface EventCategory {
  key: string;
  label: string;
  /// Types d'`audit_logs` couverts. Vide = tous (vue "Tout").
  eventTypes: string[];
}

export const EVENT_CATEGORIES: EventCategory[] = [
  { key: "all", label: "Tout", eventTypes: [] },
  {
    key: "members",
    label: "Membres",
    eventTypes: [
      "member_join",
      "member_leave",
      "member_ban",
      "member_unban",
      "member_kick",
      "member_timeout",
      "member_timeout_removed",
    ],
  },
  {
    key: "profiles",
    label: "Profils et roles",
    eventTypes: [
      "member_nickname_update",
      "member_nickname_history",
      "member_avatar_update",
      "member_roles_update",
    ],
  },
  {
    key: "voice",
    label: "Vocal",
    eventTypes: ["voice_join", "voice_leave", "voice_move"],
  },
  {
    key: "messages",
    label: "Messages",
    eventTypes: ["message_delete", "message_update", "message_delete_bulk"],
  },
  {
    key: "server",
    label: "Serveur",
    eventTypes: [
      "channel_create",
      "channel_delete",
      "channel_update",
      "role_create",
      "role_delete",
      "role_update",
      "thread_create",
      "thread_delete",
      "invite_create",
      "invite_delete",
      "guild_update",
    ],
  },
  {
    key: "admin",
    label: "Commandes admin",
    eventTypes: ["admin_command"],
  },
  {
    key: "anomalies",
    label: "Anomalies",
    eventTypes: ["anomaly_detected"],
  },
];

export interface EventLogQuery {
  guildId: string;
  eventTypes?: string[];
  from?: string | null;
  to?: string | null;
  search?: string | null;
  limit?: number;
  offset?: number;
}

export const eventLogService = {
  /** GET /api/audit-logs — page courante + total (en-tete X-Total-Count). */
  list(params: EventLogQuery): Promise<{ data: AuditLog[]; total: number }> {
    const query = q({
      guild_id: params.guildId,
      event_types: params.eventTypes?.length ? params.eventTypes.join(",") : null,
      from: params.from ?? null,
      to: params.to ?? null,
      search: params.search ?? null,
      limit: params.limit ?? 50,
      offset: params.offset ?? 0,
    });
    return httpGetWithTotal<AuditLog[]>(`/api/audit-logs${query}`);
  },
};
