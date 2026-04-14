import { httpGet, httpPatch } from "@/api/http";
import type { VoiceChannel, VoiceChannelDetail } from "@/types";

export const voiceChannelsService = {
  getAll(guildId?: string | null): Promise<VoiceChannel[]> {
    return httpGet(guildId ? `/api/voice-channels/${guildId}` : `/api/voice-channels/_all`);
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
};
