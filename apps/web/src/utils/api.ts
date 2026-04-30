// Helper pour recuperer l'URL de l'API backend depuis la config locale.
// (anciennement bridgait vers un invoke Tauri ; on lit directement
// configService desormais.)

import { configService } from "@/services/configService";

let cachedUrl: string | null = null;

export async function getApiBaseUrl(): Promise<string> {
  if (cachedUrl) return cachedUrl;
  const config = configService.getApiConfig();
  // En prod, fallback URL relative -> passe par le proxy nginx.
  // En dev, fallback localhost:3000 -> hit l'API directement.
  const fallback = import.meta.env.PROD ? "" : "http://localhost:3000";
  cachedUrl = config?.api_url || import.meta.env.VITE_API_URL || fallback;
  return cachedUrl;
}

export function resetApiBaseUrlCache() {
  cachedUrl = null;
}
