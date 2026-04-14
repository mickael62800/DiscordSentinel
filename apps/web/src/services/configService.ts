// Service de configuration locale (localStorage). Aucun appel HTTP — wrap simple
// autour des helpers de api/config.ts pour cohérence avec les autres services.

import {
  getApiConfig, setApiConfig, clearApiConfig,
  getDiscordConfig, setDiscordConfig, clearDiscordConfig,
  type ApiConfig, type DiscordConfig,
} from "@/api/config";

export const configService = {
  getApiConfig(): ApiConfig | null { return getApiConfig(); },
  saveApiConfig(apiUrl: string, apiKey: string) {
    setApiConfig({ api_url: apiUrl, api_key: apiKey });
  },
  clearApiConfig() { clearApiConfig(); },

  hasDiscordConfig(): boolean { return getDiscordConfig() !== null; },
  saveDiscordConfig(clientId: string, clientSecret: string) {
    setDiscordConfig({ client_id: clientId, client_secret: clientSecret });
  },
  clearDiscordConfig() { clearDiscordConfig(); },
};
