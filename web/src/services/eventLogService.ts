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
  /// Segment d'URL du journal (`/journal/<slug>`). Vide pour le journal global.
  slug: string;
  /// Cle RBAC : chaque journal se donne separement, comme les anciens salons
  /// Discord avaient chacun leurs permissions.
  rbacKey: string;
  /// Types d'`audit_logs` couverts. Vide = tous (journal global).
  eventTypes: string[];
}

export const EVENT_CATEGORIES: EventCategory[] = [
  { key: "all", label: "Tout", slug: "", rbacKey: "logs.journal", eventTypes: [] },
  {
    key: "members",
    label: "Membres",
    slug: "membres",
    rbacKey: "logs.journal.members",
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
    slug: "profils",
    rbacKey: "logs.journal.profiles",
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
    slug: "vocal",
    rbacKey: "logs.journal.voice",
    eventTypes: ["voice_join", "voice_leave", "voice_move"],
  },
  {
    key: "messages",
    label: "Messages",
    slug: "messages",
    rbacKey: "logs.journal.messages",
    eventTypes: ["message_delete", "message_update", "message_delete_bulk"],
  },
  {
    key: "server",
    label: "Serveur",
    slug: "serveur",
    rbacKey: "logs.journal.server",
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
    slug: "commandes",
    rbacKey: "logs.journal.admin",
    eventTypes: ["admin_command"],
  },
  {
    key: "anomalies",
    label: "Anomalies",
    slug: "anomalies",
    rbacKey: "logs.journal.anomalies",
    eventTypes: ["anomaly_detected"],
  },
  {
    // Sanctions posées par un modérateur ou par l'automod. Remplace le salon
    // Discord `sanctions_log_channel_id` : même contenu, même code couleur,
    // mais consultable, filtrable et daté.
    key: "moderation",
    label: "Modération",
    slug: "moderation",
    rbacKey: "logs.journal.moderation",
    eventTypes: ["sanction_applied", "age_verification_ban"],
  },
  {
    // Détections de l'automod. La carte Discord reste posée — elle porte les
    // boutons de vote — mais l'historique se consulte ici.
    key: "automod",
    label: "Automod",
    slug: "automod",
    rbacKey: "logs.journal.automod",
    eventTypes: ["automod_flagged"],
  },
  {
    key: "sessions",
    label: "Sessions vocales",
    slug: "sessions",
    rbacKey: "logs.journal.sessions",
    eventTypes: ["voice_session_open"],
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

/// Journal correspondant a un chemin `/journal[/slug]`. Repli sur le journal
/// global si le slug est inconnu (URL bricolee a la main).
export function categoryFromPath(path: string): EventCategory {
  const slug = path.replace(/^\/journal\/?/, "").split("/")[0] ?? "";
  return EVENT_CATEGORIES.find((c) => c.slug === slug) ?? EVENT_CATEGORIES[0];
}
