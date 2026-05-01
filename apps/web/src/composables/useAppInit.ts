import { preloadBotDefinitions } from "./useBotDefinitions";
import { preloadBotEnabledStatus } from "./useBotEnabledStatus";
import { preloadComponentVisibility } from "./useComponentVisibility";
import { preloadMyRole } from "./useMyRole";

/**
 * Prefetch des donnees stables apres login confirme.
 *
 * But : eviter que chaque page navigue refetch les memes donnees au mount.
 * Appele une fois apres login (ou changement de guild) -> tous les composables
 * singleton (useBotDefinitions, useBotEnabledStatus, useComponentVisibility,
 * useDiscordRoles...) ont deja leur cache rempli quand l'utilisateur arrive
 * sur le dashboard.
 *
 * Strategie : Promise.allSettled (un fetch echoue = pas bloquant pour les autres).
 * Backend a deja des caches Redis sur les endpoints critiques, donc cette
 * orchestration cote frontend reduit surtout la latence percue (parallelisation
 * + warm-up avant la 1ere navigation).
 */

let lastInitGuild: string | null = null;
let inFlight: Promise<void> | null = null;

export async function initAppData(guildId: string | null): Promise<void> {
  if (!guildId) return;

  // Idempotent : ne relance pas si deja init pour cette guild
  if (lastInitGuild === guildId && !inFlight) return;
  if (inFlight && lastInitGuild === guildId) return inFlight;

  lastInitGuild = guildId;
  inFlight = (async () => {
    try {
      await Promise.allSettled([
        preloadBotDefinitions(),
        preloadBotEnabledStatus(guildId),
        preloadMyRole(guildId),
        preloadComponentVisibility(guildId),
      ]);
    } finally {
      inFlight = null;
    }
  })();
  return inFlight;
}

export function resetAppInit(): void {
  lastInitGuild = null;
  inFlight = null;
}
