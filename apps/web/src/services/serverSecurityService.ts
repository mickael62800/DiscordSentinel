import { httpDelete, httpGet, httpPost } from "@/api/http";

export interface ServerEventDto {
  id: string;
  timestamp: string;
  actor: string | null;
  actor_name: string | null;
  action: string;
  target: string | null;
  severity: "info" | "warn" | "critical";
  details: unknown;
}

/**
 * Wrapper pour les endpoints /api/security/* (page Securite serveur).
 * Distinct de securityService.ts qui gere les events Discord (raid/altdetect).
 */

export interface TopIpEntry {
  client_ip: string;
  total: number;
  failed: number;
  last_seen: string;
}

export interface AuthFailureEntry {
  timestamp: string;
  status_code: number;
  method: string;
  route: string;
  client_ip: string;
  user_agent: string;
}

export interface Fail2banJail {
  name: string;
  total_banned: number;
  banned_ips: string[];
}

export interface BannedIpsResponse {
  installed: boolean;
  updated_at: string | null;
  message: string;
  jails: Fail2banJail[];
}

export interface AuditEntry {
  id: string;
  guild_id: string;
  event_type: string;
  actor_id: string | null;
  actor_name: string | null;
  target_id: string | null;
  target_name: string | null;
  details: unknown;
  created_at: string;
}

export interface TlsCertInfo {
  domain: string;
  issuer: string;
  subject: string;
  not_before: string;
  not_after: string;
  days_until_expiry: number;
  is_expired: boolean;
  is_warning: boolean;
}

export type SecurityWindow = "1h" | "24h" | "7d";

export interface CleanupResponse {
  deleted_api_logs: number;
  deleted_audit_logs: number;
  message: string;
}

export const serverSecurityService = {
  topIps(window: SecurityWindow = "1h", limit = 20): Promise<TopIpEntry[]> {
    return httpGet(`/api/security/top-ips?window=${window}&limit=${limit}`);
  },
  authFailures(window: SecurityWindow = "24h", limit = 100): Promise<AuthFailureEntry[]> {
    return httpGet(`/api/security/auth-failures?window=${window}&limit=${limit}`);
  },
  bannedIps(): Promise<BannedIpsResponse> {
    return httpGet("/api/security/banned-ips");
  },
  auditLogs(params: { guild_id?: string; event_type_prefix?: string; limit?: number } = {}): Promise<AuditEntry[]> {
    const u = new URLSearchParams();
    if (params.guild_id) u.set("guild_id", params.guild_id);
    if (params.event_type_prefix) u.set("event_type_prefix", params.event_type_prefix);
    u.set("limit", String(params.limit ?? 100));
    return httpGet(`/api/security/audit-logs?${u.toString()}`);
  },
  tlsCert(): Promise<TlsCertInfo> {
    return httpGet("/api/security/tls-cert");
  },
  banIp(ip: string, reason?: string): Promise<{ ok: boolean; message: string }> {
    return httpPost("/api/security/ban-ip", { ip, reason });
  },
  unbanIp(ip: string, reason?: string): Promise<{ ok: boolean; message: string }> {
    return httpPost("/api/security/unban-ip", { ip, reason });
  },
  serverEvents(params: { action_prefix?: string; severity?: string; limit?: number } = {}): Promise<ServerEventDto[]> {
    const u = new URLSearchParams();
    if (params.action_prefix) u.set("action_prefix", params.action_prefix);
    if (params.severity) u.set("severity", params.severity);
    u.set("limit", String(params.limit ?? 100));
    return httpGet(`/api/security/server-events?${u.toString()}`);
  },
  cleanup(opts: { older_than_days?: number; include_audit_logs?: boolean } = {}): Promise<CleanupResponse> {
    const u = new URLSearchParams();
    if (opts.older_than_days !== undefined) u.set("older_than_days", String(opts.older_than_days));
    if (opts.include_audit_logs !== undefined) u.set("include_audit_logs", String(opts.include_audit_logs));
    const qs = u.toString();
    return httpDelete(`/api/security/cleanup${qs ? `?${qs}` : ""}`);
  },
};
