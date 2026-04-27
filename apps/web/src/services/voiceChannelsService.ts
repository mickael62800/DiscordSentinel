import { httpDelete, httpGet, httpPatch, httpPost } from "@/api/http";
import type { VoiceChannel, VoiceChannelDetail } from "@/types";
import type {
  AddCoAdminPayload,
  AddWhitelistPayload,
  BanFromChannelPayload,
  ChannelBan,
  CoAdmin,
  CreateInvitePayload,
  CreateThemePayload,
  InviteLink,
  TransferOwnershipPayload,
  UpdateThemePayload,
  VoiceChannelTheme,
  WhitelistEntry,
} from "@/types/voice-extended";
import { q } from "./_query";

// ── Themes ────────────────────────────────────────────────────
export const voiceThemesService = {
  list(guildId: string): Promise<VoiceChannelTheme[]> {
    return httpGet(`/api/voice-channels/themes/${guildId}`);
  },
  create(guildId: string, body: CreateThemePayload): Promise<VoiceChannelTheme> {
    return httpPost(`/api/voice-channels/themes/${guildId}`, body);
  },
  update(
    guildId: string,
    themeId: string,
    body: UpdateThemePayload,
  ): Promise<VoiceChannelTheme> {
    return httpPatch(`/api/voice-channels/themes/${guildId}/${themeId}`, body);
  },
  remove(guildId: string, themeId: string): Promise<unknown> {
    return httpDelete(`/api/voice-channels/themes/${guildId}/${themeId}`);
  },
};

// ── Whitelists / Bans / Invites / Co-admins / Transfer ────────
export const voiceManageService = {
  // Whitelist par owner.
  getWhitelist(guildId: string, ownerId: string): Promise<WhitelistEntry[]> {
    return httpGet(`/api/voice-channels/whitelist/${guildId}/${ownerId}`);
  },
  addWhitelist(body: AddWhitelistPayload): Promise<WhitelistEntry> {
    return httpPost(`/api/voice-channels/whitelist`, body);
  },
  removeWhitelist(
    guildId: string,
    ownerId: string,
    targetId: string,
  ): Promise<unknown> {
    return httpDelete(
      `/api/voice-channels/whitelist/${guildId}/${ownerId}/${targetId}`,
    );
  },
  // Bans par channel.
  banFromChannel(
    channelId: string,
    body: BanFromChannelPayload,
  ): Promise<ChannelBan> {
    return httpPost(`/api/voice-channels/by-channel/${channelId}/bans`, body);
  },
  unbanFromChannel(channelId: string, userId: string): Promise<unknown> {
    return httpDelete(
      `/api/voice-channels/by-channel/${channelId}/bans/${userId}`,
    );
  },
  // Invites custom par channel.
  listInvites(channelId: string): Promise<InviteLink[]> {
    return httpGet(`/api/voice-channels/by-channel/${channelId}/invites`);
  },
  createInvite(
    channelId: string,
    body: CreateInvitePayload,
  ): Promise<InviteLink> {
    return httpPost(`/api/voice-channels/by-channel/${channelId}/invites`, body);
  },
  revokeInvite(channelId: string, linkId: string): Promise<unknown> {
    return httpDelete(
      `/api/voice-channels/by-channel/${channelId}/invites/${linkId}`,
    );
  },
  // Co-admins.
  addCoAdmin(channelId: string, body: AddCoAdminPayload): Promise<CoAdmin> {
    return httpPost(
      `/api/voice-channels/by-channel/${channelId}/co-admins`,
      body,
    );
  },
  removeCoAdmin(channelId: string, userId: string): Promise<unknown> {
    return httpDelete(
      `/api/voice-channels/by-channel/${channelId}/co-admins/${userId}`,
    );
  },
  // Transfer ownership.
  transferOwnership(
    channelId: string,
    body: TransferOwnershipPayload,
  ): Promise<unknown> {
    return httpPatch(
      `/api/voice-channels/by-channel/${channelId}/transfer`,
      body,
    );
  },
};

export const voiceChannelsService = {
  getAll(guildId?: string | null): Promise<VoiceChannel[]> {
    return httpGet(guildId ? `/api/voice-channels/${guildId}` : `/api/voice-channels/_all`);
  },
  /**
   * Historique des salons fermes (channel_status = 'closed').
   * Limite par defaut backend : 100, max 500.
   */
  getHistory(guildId: string, limit?: number): Promise<VoiceChannel[]> {
    return httpGet(`/api/voice-channels/${guildId}/history${q({ limit })}`);
  },
  getDetail(channelId: string): Promise<VoiceChannelDetail> {
    return httpGet(`/api/voice-channels/by-channel/${channelId}`);
  },
  /**
   * Ferme (soft-delete) un salon vocal dans la DB. Utile pour nettoyer
   * les lignes fantomes (salons Discord supprimes sans que le bot ait
   * pu appeler son propre nettoyage — restart, crash, etc.).
   */
  close(channelId: string): Promise<unknown> {
    return httpPatch(`/api/voice-channels/by-channel/${channelId}/close`);
  },
  purge(channelId: string): Promise<unknown> {
    return httpDelete(`/api/voice-channels/by-channel/${channelId}/purge`);
  },
  purgeHistory(guildId: string): Promise<{ deleted: number }> {
    return httpDelete(`/api/voice-channels/${guildId}/history`);
  },
  getEvents(channelId: string, limit = 200): Promise<VoiceChannelEvent[]> {
    return httpGet(`/api/voice-channels/by-channel/${channelId}/events${q({ limit })}`);
  },
};

export interface VoiceChannelEvent {
  id: string;
  guild_id: string;
  event_type: string;
  actor_id: string | null;
  actor_name: string | null;
  channel_id: string | null;
  channel_name: string | null;
  details: Record<string, unknown>;
  created_at: string;
}
