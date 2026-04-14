// Helper pour recuperer l'URL de l'API backend depuis la config locale.
// (anciennement bridgait vers un invoke Tauri ; on lit directement
// configService desormais.)

import { configService } from "@/services/configService";

let cachedUrl: string | null = null;

export async function getApiBaseUrl(): Promise<string> {
  if (cachedUrl) return cachedUrl;
  const config = configService.getApiConfig();
  cachedUrl = config?.api_url || "http://localhost:3000";
  return cachedUrl;
}

export function resetApiBaseUrlCache() {
  cachedUrl = null;
}
