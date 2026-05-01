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

// SSH failures
export interface SshFailureEntry {
  timestamp: string;
  user: string;
  ip: string;
  message: string;
}
export interface SshFailuresResponse {
  updated_at: string;
  total_24h: number;
  entries: SshFailureEntry[];
}

// Disk trend
export interface DiskTrendPoint {
  timestamp: string;
  mount: string;
  used_gb: number;
  total_gb: number;
  usage_pct: number;
}
export interface DiskTrendResponse {
  updated_at: string;
  points: DiskTrendPoint[];
}

// Active connections
export interface ConnectionEntry {
  state: string;
  local_addr: string;
  remote_addr: string;
  process: string | null;
}
export interface ConnectionsResponse {
  updated_at: string;
  total: number;
  connections: ConnectionEntry[];
}

// Open ports
export interface OpenPort {
  port: number;
  protocol: string;
  service: string | null;
  expected: boolean;
}
export interface OpenPortsResponse {
  updated_at: string;
  ports: OpenPort[];
  unexpected_count: number;
}

// Trivy
export interface TrivyVuln {
  image: string;
  cve: string;
  severity: string;
  package: string | null;
  fixed_version: string | null;
}
export interface TrivyResponse {
  updated_at: string;
  critical: number;
  high: number;
  medium: number;
  low: number;
  vulnerabilities: TrivyVuln[];
}

export interface SuccessfulLoginEntry {
  timestamp: string;
  discord_user_id: string;
  username: string | null;
  client_ip: string | null;
  user_agent: string | null;
}

export interface TrafficDatapoint {
  timestamp: string;
  total: number;
  errors: number;
}

export interface TrafficTrendResponse {
  datapoints: TrafficDatapoint[];
  baseline_avg: number;
  peak: number;
  peak_at: string | null;
  alert: boolean;
  alert_reason: string | null;
}

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
  trafficTrend(window: SecurityWindow | "6h" = "24h", bucket_minutes = 5): Promise<TrafficTrendResponse> {
    return httpGet(`/api/security/traffic-trend?window=${window}&bucket_minutes=${bucket_minutes}`);
  },
  lastLogins(limit = 20): Promise<SuccessfulLoginEntry[]> {
    return httpGet(`/api/security/last-logins?limit=${limit}`);
  },
  sshFailures(): Promise<SshFailuresResponse> {
    return httpGet("/api/security/ssh-failures");
  },
  diskTrend(): Promise<DiskTrendResponse> {
    return httpGet("/api/security/disk-trend");
  },
  connections(): Promise<ConnectionsResponse> {
    return httpGet("/api/security/connections");
  },
  openPorts(): Promise<OpenPortsResponse> {
    return httpGet("/api/security/open-ports");
  },
  trivy(): Promise<TrivyResponse> {
    return httpGet("/api/security/trivy");
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
