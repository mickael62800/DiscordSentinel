import { onUnmounted } from "vue";
import { useRealtime } from "./useRealtime";
import type { UnlistenFn } from "@tauri-apps/api/event";

/**
 * Composable qui rafraichit automatiquement des donnees quand un event WebSocket arrive.
 * Utilise un debounce pour eviter les appels multiples lors de rafales d'events.
 *
 * @param eventTypes - Les types d'events WS a ecouter (ex: ["voice_channel_created", "voice_channel_closed"])
 * @param callback - La fonction de refresh a appeler (ex: fetchChannels)
 * @param options - Options optionnelles (debounceMs: delai en ms, defaut 500)
 */
export function useRealtimeRefresh(
  eventTypes: string[],
  callback: () => void,
  options?: { debounceMs?: number },
): void {
  const { onEvent } = useRealtime();
  const debounceMs = options?.debounceMs ?? 500;
  const unlisteners: UnlistenFn[] = [];
  let timer: ReturnType<typeof setTimeout> | null = null;

  function debouncedCallback() {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      callback();
      timer = null;
    }, debounceMs);
  }

  for (const eventType of eventTypes) {
    onEvent(eventType, debouncedCallback).then((unlisten) => {
      unlisteners.push(unlisten);
    });
  }

  onUnmounted(() => {
    if (timer) clearTimeout(timer);
    for (const unlisten of unlisteners) {
      unlisten();
    }
  });
}
