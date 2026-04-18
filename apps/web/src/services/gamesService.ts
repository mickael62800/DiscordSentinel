import { apiBase, httpGet, httpPost, httpPatch, httpDelete } from "@/api/http";
import { getApiConfig, getDiscordToken } from "@/api/config";

export interface Game {
  id: string;
  guild_id: string;
  game_name: string;
  created_by: string;
  created_at: string;
  emoji: string | null;
  category: string | null;
}

export interface GamePanel {
  id: string;
  guild_id: string;
  channel_id: string;
  message_id: string;
  category: string | null;
}

export interface UploadEmojiResponse {
  emoji: string;
  emoji_id: string;
  name: string;
  animated: boolean;
}

export interface CreateGamePayload {
  guild_id: string;
  game_name: string;
  created_by: string;
  emoji?: string | null;
  category?: string | null;
}

export interface UpdateGamePayload {
  game_name?: string;
  emoji?: string | null;
  category?: string | null;
}

export const gamesService = {
  list(guildId: string): Promise<Game[]> {
    return httpGet(`/api/games/${guildId}`);
  },
  create(payload: CreateGamePayload): Promise<Game> {
    return httpPost(`/api/games`, payload);
  },
  update(guildId: string, gameId: string, payload: UpdateGamePayload): Promise<Game> {
    return httpPatch(`/api/games/${guildId}/${gameId}`, payload);
  },
  delete(guildId: string, gameId: string): Promise<void> {
    return httpDelete(`/api/games/${guildId}/${gameId}`);
  },
  async getSubscribers(guildId: string, gameId: string): Promise<{ user_id: string }[]> {
    return httpGet(`/api/games/${guildId}/${gameId}/subscribers`);
  },
  listPanels(guildId: string): Promise<GamePanel[]> {
    return httpGet(`/api/games/${guildId}/panels`);
  },

  /**
   * Upload multipart d'un emoji (name + image). Passe par fetch direct pour
   * conserver les headers standards mais avec multipart.
   */
  async uploadEmoji(
    guildId: string,
    name: string,
    image: Blob,
  ): Promise<UploadEmojiResponse> {
    const fd = new FormData();
    fd.append("name", name);
    fd.append("image", image);

    const cfg = getApiConfig();
    const headers: Record<string, string> = {};
    if (cfg?.api_key) headers["Authorization"] = `Bearer ${cfg.api_key}`;
    const tok = getDiscordToken();
    if (tok) headers["X-Discord-Token"] = tok;

    const resp = await fetch(`${apiBase()}/api/games/${guildId}/upload-emoji`, {
      method: "POST",
      headers,
      body: fd,
    });

    if (!resp.ok) {
      const body = await resp.text().catch(() => "");
      throw new Error(`API error ${resp.status}: ${body}`);
    }
    return resp.json();
  },
};
