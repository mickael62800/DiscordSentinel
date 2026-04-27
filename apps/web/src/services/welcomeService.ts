import { httpGet, httpPut } from "@/api/http";
import type { WelcomeConfig, SaveWelcomeConfigParams } from "@/types/welcome";

export const welcomeService = {
  getConfig(guildId: string): Promise<WelcomeConfig> {
    return httpGet(`/api/welcome/${guildId}`);
  },
  saveConfig(guildId: string, body: SaveWelcomeConfigParams): Promise<WelcomeConfig> {
    return httpPut(`/api/welcome/${guildId}`, body);
  },
};
