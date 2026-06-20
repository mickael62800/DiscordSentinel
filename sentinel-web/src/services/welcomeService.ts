import { httpGet, httpPut, httpPost } from "@/api/http";
import type { WelcomeConfig, SaveWelcomeConfigParams } from "@/types/welcome";

export const welcomeService = {
  getConfig(guildId: string): Promise<WelcomeConfig> {
    return httpGet(`/api/welcome/${guildId}`);
  },
  saveConfig(guildId: string, body: SaveWelcomeConfigParams): Promise<WelcomeConfig> {
    return httpPut(`/api/welcome/${guildId}`, body);
  },
  // Demande au bot de (re)poster le panneau de reglement (texte + bouton) dans
  // le salon configure.
  publishRules(guildId: string): Promise<{ ok: boolean }> {
    return httpPost(`/api/welcome/${guildId}/rules/publish`, {});
  },
};
