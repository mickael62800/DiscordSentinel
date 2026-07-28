// Auth Discord cote web : redirige le navigateur vers la route backend
// `/auth/discord/authorize` qui genere le state CSRF et renvoie sur Discord.
// Le backend detient le client_secret, le front n'en voit jamais rien.

import {
  getDiscordUser, setDiscordUser,
  clearDiscordToken,
  type DiscordUser,
} from "@/api/config";
import { getApiBaseUrl } from "@/utils/api";

export const authService = {
  getCurrentUser(): DiscordUser | null { return getDiscordUser(); },

  async discordLogin(): Promise<DiscordUser> {
    const base = await getApiBaseUrl();
    // Navigation complete du navigateur vers Discord via le backend.
    // La Promise ne resolve jamais : soit la page change, soit on throw.
    window.location.href = `${base.replace(/\/$/, "")}/auth/discord/authorize`;
    // Pour satisfaire le typage (la page est en train de naviguer) :
    return new Promise<DiscordUser>(() => {});
  },

  logout() {
    setDiscordUser(null);
    clearDiscordToken();
  },
};
