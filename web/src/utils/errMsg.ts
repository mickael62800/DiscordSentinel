/**
 * Extrait un message lisible d'une erreur `unknown` (catch TS).
 * Remplace le pattern `catch (e: any) { e?.message ?? e }` sans `any`.
 */
export function errMsg(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
