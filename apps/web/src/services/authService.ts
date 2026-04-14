// Auth Discord cote web : pas de flux OAuth (pas de Tauri), on lit/ecrit
// l'utilisateur en cache localStorage. La fonction login() echoue tant que le
// flux OAuth web n'est pas implemente (cf. SetupPage).

import {
  getDiscordUser, setDiscordUser,
  clearApiConfig, clearDiscordToken,
  type DiscordUser,
} from "@/api/config";

export const authService = {
  getCurrentUser(): DiscordUser | null { return getDiscordUser(); },

  async discordLogin(): Promise<DiscordUser> {
    const u = getDiscordUser();
    if (!u) throw new Error("OAuth web non implemente. Configurez l'API key via la page Setup.");
    return u;
  },

  logout() {
    setDiscordUser(null);
    clearDiscordToken();
    clearApiConfig();
  },
};
