// Serveurs de jeu de la plateforme Nexus (game-portal).
//
// Passe par la passerelle /nexus-api : l'autorisation est verifiee cote
// serveur par nginx (gate RBAC `nexus.access`) avant d'atteindre nexus-api.

import { nexusDelete, nexusGet, nexusPost } from "@/api/nexusHttp";

/** Etats possibles d'un serveur, tels que renvoyes par l'API. */
export type GameServerStatus =
  | "created"
  | "starting"
  | "running"
  | "stopping"
  | "stopped"
  | "error"
  | "deleted";

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
  text_channel_id: string | null;
  voice_channel_id: string | null;
  ip_reveal_at: string | null;
  ip_revealed: boolean;
}

export interface GameTemplate {
  id: string;
  slug: string;
  name: string;
  description?: string | null;
  icon?: string | null;
  category?: string | null;
}

export const nexusGamesService = {
  /** GET /api/games/{guild}/servers */
  listServers(guildId: string): Promise<GameServer[]> {
    return nexusGet<GameServer[]>(`/api/games/${encodeURIComponent(guildId)}/servers`, guildId);
  },

  /** GET /api/games/{guild}/templates — catalogue des jeux disponibles. */
  listTemplates(guildId: string): Promise<GameTemplate[]> {
    return nexusGet<GameTemplate[]>(`/api/games/${encodeURIComponent(guildId)}/templates`, guildId);
  },

  /**
   * POST /api/games/servers/{id}/start
   * `actorId` sert a tracer qui a declenche l'action dans le journal d'audit.
   */
  start(guildId: string, serverId: string, actorId: string): Promise<void> {
    return nexusPost<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}/start?actor_id=${encodeURIComponent(actorId)}`,
      guildId,
    );
  },

  /** POST /api/games/servers/{id}/stop */
  stop(guildId: string, serverId: string, actorId: string): Promise<void> {
    return nexusPost<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}/stop?actor_id=${encodeURIComponent(actorId)}`,
      guildId,
    );
  },

  /** POST /api/games/servers/{id}/restart */
  restart(guildId: string, serverId: string, actorId: string): Promise<void> {
    return nexusPost<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}/restart?actor_id=${encodeURIComponent(actorId)}`,
      guildId,
    );
  },

  /** DELETE /api/games/servers/{id} */
  remove(guildId: string, serverId: string, actorId: string): Promise<void> {
    return nexusDelete<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}?actor_id=${encodeURIComponent(actorId)}`,
      guildId,
    );
  },
};
