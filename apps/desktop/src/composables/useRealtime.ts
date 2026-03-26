import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

const connected = ref(false);
const wsUrl = ref("");
let initialized = false;
const unlisteners: UnlistenFn[] = [];

export interface WsEvent {
  event: string;
  data: unknown;
}

export function useRealtime() {
  async function connect() {
    try {
      await invoke("ws_connect");
    } catch {
      // Will retry via auto-reconnect in Rust
    }
  }

  async function disconnect() {
    try {
      await invoke("ws_disconnect");
    } catch {
      // Ignore disconnect errors
    }
    connected.value = false;
  }

  async function getStatus() {
    const status = await invoke<{ connected: boolean; url: string }>("ws_status");
    connected.value = status.connected;
    wsUrl.value = status.url;
    return status;
  }

  async function init() {
    if (initialized) return;
    initialized = true;

    const u1 = await listen("ws:connected", (event) => {
      const payload = event.payload as { connected: boolean; url: string };
      connected.value = payload.connected;
      wsUrl.value = payload.url;
    });
    unlisteners.push(u1);

    const u2 = await listen("ws:disconnected", () => {
      connected.value = false;
    });
    unlisteners.push(u2);

    await connect();
  }

  function cleanup() {
    for (const unlisten of unlisteners) {
      unlisten();
    }
    unlisteners.length = 0;
    initialized = false;
  }

  async function onEvent(eventType: string, callback: (data: unknown) => void): Promise<UnlistenFn> {
    const unlisten = await listen(`ws:${eventType}`, (event) => {
      callback(event.payload);
    });
    unlisteners.push(unlisten);
    return unlisten;
  }

  return { connected, wsUrl, init, connect, disconnect, cleanup, getStatus, onEvent };
}
