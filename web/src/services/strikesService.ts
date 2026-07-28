import { httpDelete, httpGet, httpPost, httpPut } from "@/api/http";
import type {
  AddStrikePayload,
  SaveStrikeConfigPayload,
  StrikeConfig,
  StrikeResult,
  UserStrike,
} from "@/types/strikes";

export const strikesService = {
  getConfig(guildId: string): Promise<StrikeConfig> {
    return httpGet(`/api/strikes/config/${guildId}`);
  },
  saveConfig(guildId: string, body: SaveStrikeConfigPayload): Promise<StrikeConfig> {
    return httpPut(`/api/strikes/config/${guildId}`, body);
  },
  getActiveStrikes(guildId: string, userId: string): Promise<UserStrike[]> {
    return httpGet(`/api/strikes/${guildId}/${userId}`);
  },
  addStrike(body: AddStrikePayload): Promise<StrikeResult> {
    return httpPost("/api/strikes", body);
  },
  resetStrikes(guildId: string, userId: string): Promise<unknown> {
    return httpDelete(`/api/strikes/${guildId}/${userId}`);
  },
};
