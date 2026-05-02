//! Service Game Portal — appels HTTP vers /api/games/...
//!
//! Architecture :
//!  - Templates : catalogue Docker images whitelistees par guild.
//!  - Servers : instances avec lifecycle complet (create/start/stop/...).
//!  - Logs / stats / RCON.

import { httpDelete, httpGet, httpPost, httpPut } from "@/api/http";

// ── Types ──────────────────────────────────────────────────────────────

export type GameServerStatus =
  | "created"
  | "starting"
  | "running"
  | "stopping"
  | "stopped"
  | "error"
  | "deleted";

export type ConfigFieldType = "text" | "number" | "enum" | "boolean";

export interface ConfigField {
  key: string;
  label: string;
  type: ConfigFieldType;
  default?: string | number | boolean;
  options?: string[];
  min?: number;
  max?: number;
  max_length?: number;
}

export interface GameTemplate {
  id: string;
  slug: string;
  name: string;
  description: string | null;
  category: string | null;
  icon: string | null;
  accent_color: string | null;
  container_port: number;
  port_protocol: "tcp" | "udp";
  default_memory_mb: number;
  min_memory_mb: number;
  max_memory_mb: number;
  config_schema: ConfigField[];
  supports_rcon: boolean;
  supports_mods: boolean;
  idle_shutdown_days: number;
}

export interface GameServer {
  id: string;
  guild_id: string;
  template_id: string;
  name: string;
  status: GameServerStatus;
  host_port: number | null;
  rcon_port: number | null;
  allocated_memory_mb: number;
  owner_user_id: string;
  last_active_at: string | null;
  last_player_count: number;
  last_error: string | null;
  created_at: string;
  started_at: string | null;
  stopped_at: string | null;
}

export interface GameServerDetail {
  server: GameServer;
  config: Record<string, string>;
}

export interface GameServerStats {
  cpu_percent: number;
  memory_used_mb: number;
  memory_limit_mb: number;
  network_rx_bytes: number;
  network_tx_bytes: number;
}

export interface CreateGameServerPayload {
  template_slug: string;
  name: string;
  memory_mb?: number;
  owner_user_id: string;
  config?: Record<string, string>;
}

export interface RconResponse {
  response: string;
}

export interface PlayerSession {
  id: string;
  server_id: string;
  player_name: string;
  joined_at: string;
  left_at: string | null;
  duration_seconds: number | null;
}

// ── Service ────────────────────────────────────────────────────────────

export const gamePortalService = {
  // Templates
  listTemplates(guildId: string): Promise<GameTemplate[]> {
    return httpGet(`/api/games/${guildId}/templates`);
  },
  getTemplate(id: string): Promise<GameTemplate> {
    return httpGet(`/api/games/templates/${id}`);
  },

  // Servers
  listServers(guildId: string): Promise<GameServer[]> {
    return httpGet(`/api/games/${guildId}/servers`);
  },
  getServer(serverId: string): Promise<GameServerDetail> {
    return httpGet(`/api/games/servers/${serverId}`);
  },
  createServer(
    guildId: string,
    payload: CreateGameServerPayload,
  ): Promise<GameServer> {
    return httpPost(`/api/games/${guildId}/servers`, payload);
  },
  deleteServer(serverId: string, actorId?: string): Promise<void> {
    const q = actorId ? `?actor_id=${encodeURIComponent(actorId)}` : "";
    return httpDelete(`/api/games/servers/${serverId}${q}`);
  },

  // Lifecycle
  startServer(serverId: string, actorId?: string): Promise<void> {
    const q = actorId ? `?actor_id=${encodeURIComponent(actorId)}` : "";
    return httpPost(`/api/games/servers/${serverId}/start${q}`, {});
  },
  stopServer(serverId: string, actorId?: string): Promise<void> {
    const q = actorId ? `?actor_id=${encodeURIComponent(actorId)}` : "";
    return httpPost(`/api/games/servers/${serverId}/stop${q}`, {});
  },
  restartServer(serverId: string, actorId?: string): Promise<void> {
    const q = actorId ? `?actor_id=${encodeURIComponent(actorId)}` : "";
    return httpPost(`/api/games/servers/${serverId}/restart${q}`, {});
  },

  // Observabilite
  getLogs(serverId: string, lines = 200): Promise<string[]> {
    return httpGet(`/api/games/servers/${serverId}/logs?lines=${lines}`);
  },
  getStats(serverId: string): Promise<GameServerStats> {
    return httpGet(`/api/games/servers/${serverId}/stats`);
  },

  // Config
  updateConfig(
    serverId: string,
    config: Record<string, string>,
    actorId?: string,
  ): Promise<void> {
    const q = actorId ? `?actor_id=${encodeURIComponent(actorId)}` : "";
    return httpPut(`/api/games/servers/${serverId}/config${q}`, { config });
  },

  // RCON
  executeCommand(
    serverId: string,
    command: string,
    actorId?: string,
  ): Promise<RconResponse> {
    const q = actorId ? `?actor_id=${encodeURIComponent(actorId)}` : "";
    return httpPost(`/api/games/servers/${serverId}/command${q}`, { command });
  },

  // Sessions joueurs
  listSessions(
    serverId: string,
    limit = 100,
    offset = 0,
  ): Promise<PlayerSession[]> {
    return httpGet(
      `/api/games/servers/${serverId}/sessions?limit=${limit}&offset=${offset}`,
    );
  },
};
