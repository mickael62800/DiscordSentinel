import { storeToRefs } from "pinia";
import { useRealtimeStore } from "@/stores/realtimeStore";
import type { UnlistenFn } from "@/api/events-api";

export interface WsEvent {
  event: string;
  data: unknown;
}

export function useRealtime() {
  const store = useRealtimeStore();
  const { connected, wsUrl } = storeToRefs(store);

  return {
    connected,
    wsUrl,
    init: store.init,
    connect: store.connect,
    disconnect: store.disconnect,
    cleanup: store.cleanup,
    getStatus: store.getStatus,
    onEvent: store.onEvent as (eventType: string, callback: (data: unknown) => void) => Promise<UnlistenFn>,
  };
}
