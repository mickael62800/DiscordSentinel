import { httpGet, httpDelete, httpPost } from "@/api/http";
import type { WatchedUser, UserDossier } from "@/types";
import { q } from "./_query";

export const watchedUsersService = {
  getAll(guildId?: string | null): Promise<WatchedUser[]> {
    return httpGet(`/api/watched-users${q({ guild_id: guildId ?? null })}`);
  },
  getDossier(guildId: string, userId: string): Promise<UserDossier> {
    return httpGet(`/api/watched-users/${guildId}/${userId}`);
  },
  remove(guildId: string, userId: string): Promise<void> {
    return httpDelete(`/api/watched-users/${guildId}/${userId}`);
  },
  add(guildId: string, userId: string, username: string, reason: string): Promise<unknown> {
    return httpPost("/api/watched-users", {
      guild_id: guildId, user_id: userId, username, reason,
    });
  },
};
