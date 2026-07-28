// Bus d'evenements local. Remplace listen() par un bus
// d'evenements alimente par le WebSocket realtime.

import { on } from "./events";

export type UnlistenFn = () => void;
export interface Event<T> { event: string; windowLabel: string; id: number; payload: T }

export async function listen<T = unknown>(
  name: string,
  handler: (event: Event<T>) => void,
): Promise<UnlistenFn> {
  return on(name, (ev) => {
    handler({ event: name, windowLabel: "main", id: 0, payload: ev.payload as T });
  });
}

export async function once<T = unknown>(
  name: string,
  handler: (event: Event<T>) => void,
): Promise<UnlistenFn> {
  const unlisten = await listen<T>(name, (ev) => { unlisten(); handler(ev); });
  return unlisten;
}

export async function emit(_event: string, _payload?: unknown): Promise<void> {
  // Emit cote client uniquement : no-op dans la version web.
}
