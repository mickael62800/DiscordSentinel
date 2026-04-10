import { invoke } from "@tauri-apps/api/core";
import type { ApiConfig } from "../types";

let cachedUrl: string | null = null;

export async function getApiBaseUrl(): Promise<string> {
  if (cachedUrl) return cachedUrl;
  try {
    const config = await invoke<ApiConfig | null>("get_api_config");
    cachedUrl = config?.api_url || "http://localhost:3000";
  } catch {
    cachedUrl = "http://localhost:3000";
  }
  return cachedUrl;
}

export function resetApiBaseUrlCache() {
  cachedUrl = null;
}
