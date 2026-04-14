import { httpGet } from "@/api/http";
import type { VoiceChannel, VoiceChannelDetail } from "@/types";

export const voiceChannelsService = {
  getAll(guildId?: string | null): Promise<VoiceChannel[]> {
    return httpGet(guildId ? `/api/voice-channels/${guildId}` : `/api/voice-channels/_all`);
  },
  getDetail(channelId: string): Promise<VoiceChannelDetail> {
    return httpGet(`/api/voice-channels/by-channel/${channelId}`);
  },
};
