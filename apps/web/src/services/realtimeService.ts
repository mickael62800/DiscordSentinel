// Singleton WebSocket realtime — alimente le bus d'evenements local (api/events.ts).
// Re-publie chaque frame WS sous la cle ws:<event_name>, ce qui permet aux composables
// (useRealtime / useRealtimeRefresh) de s'abonner via listen("ws:<event>").

import { getApiConfig, getDiscordToken } from "@/api/config";
import { emit } from "@/api/events";

let ws: WebSocket | null = null;
let wsUrl = "";
let wsConnected = false;

function deriveGatewayWs(apiUrl: string, apiKey: string, discordToken: string): string {
  try {
    const u = new URL(apiUrl);
    // En prod, le proxy nginx route /ws vers le gateway sur le meme domaine.
    // En dev, gateway = port API+1.
    const isProd = u.hostname !== "localhost" && u.hostname !== "127.0.0.1";
    const port = isProd
      ? (u.port || (u.protocol === "https:" ? "443" : "80"))
      : (u.port ? String(Number(u.port) + 1) : "3001");
    const scheme = u.protocol === "https:" ? "wss" : "ws";
    const base = `${scheme}://${u.hostname}:${port}/ws`;
    // 2 modes auth :
    //   - apiKey -> ?token= (clients internes, bot, workers)
    //   - discordToken -> ?discord_token= (utilisateurs web apres OAuth)
    if (apiKey) return `${base}?token=${encodeURIComponent(apiKey)}`;
    if (discordToken) return `${base}?discord_token=${encodeURIComponent(discordToken)}`;
    return base;
  } catch {
    return "";
  }
}

export const realtimeService = {
  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      const cfg = getApiConfig();
      if (!cfg?.api_url) { reject(new Error("API not configured")); return; }
      this.disconnect();
      const url = deriveGatewayWs(cfg.api_url, cfg.api_key || "", getDiscordToken());
      wsUrl = url;
      try {
        ws = new WebSocket(url);
      } catch (e) { reject(e); return; }
      ws.onopen = () => {
        wsConnected = true;
        emit("ws:connected", { connected: true, url: wsUrl });
        resolve();
      };
      ws.onclose = () => {
        wsConnected = false;
        emit("ws:disconnected", null);
      };
      ws.onerror = () => {
        // laisse onclose gerer
      };
      ws.onmessage = (ev) => {
        try {
          const msg = JSON.parse(ev.data as string);
          if (msg && typeof msg.event === "string") {
            emit(`ws:${msg.event}`, msg.data);
          }
        } catch { /* ignore */ }
      };
    });
  },

  disconnect() {
    if (ws) { try { ws.close(); } catch { /* ignore */ } }
    ws = null;
    wsConnected = false;
  },

  status(): { connected: boolean; url: string } {
    return { connected: wsConnected, url: wsUrl };
  },
};
