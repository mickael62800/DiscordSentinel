// Bus d'evenements local. Le WebSocket realtime (services/realtimeService.ts)
// republie ses frames via emit("ws:<event>", data) pour que useRealtime /
// useRealtimeRefresh puissent s'y abonner.

type Handler = (event: { payload: unknown }) => void;
const handlers: Map<string, Set<Handler>> = new Map();

export function emit(name: string, payload: unknown) {
  const set = handlers.get(name);
  if (!set) return;
  for (const h of set) { try { h({ payload }); } catch { /* ignore */ } }
}

export function on(name: string, handler: Handler): () => void {
  let set = handlers.get(name);
  if (!set) { set = new Set(); handlers.set(name, set); }
  set.add(handler);
  return () => { set!.delete(handler); };
}
